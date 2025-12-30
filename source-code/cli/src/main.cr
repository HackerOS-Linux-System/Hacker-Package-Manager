require "http/client"
require "json"
require "file_utils"
require "process"
require "digest/sha256"

module HPM
  VERSION = "0.3"
  STORE_PATH = "/usr/lib/HackerOS/hpm/store/"
  BACKEND_PATH = "#{ENV["HOME"]}/.hackeros/hpm/bin/backend"
  REPO_JSON_URL = "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/Community/repo.json"
  VERSION_URL = "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/Community/hpm-version.hacker"
  LOCAL_VERSION_FILE = "/usr/lib/HackerOS/hpm/version.json"
  RELEASES_BASE = "https://github.com/HackerOS-Linux-System/Hacker-Package-Manager/releases/download/v"
  STATE_PATH = "/var/lib/hpm/state.json"

  def self.download_file(url : String, path : String)
    HTTP::Client.get(url) do |response|
      File.write(path, response.body_io)
    end
  end

  def self.compute_sha256(path : String) : String
    Digest::SHA256.hexdigest(File.read(path))
  end

  def self.refresh
    puts "Refreshing package index..."
    temp_path = "/tmp/repo.json"
    download_file(REPO_JSON_URL, temp_path)
    FileUtils.mv(temp_path, "/usr/lib/HackerOS/hpm/repo.json")
    puts "Package index refreshed."
  end

  def self.resolve_deps(repo : Hash(String, JSON::Any), package_name : String, installed : Set(String), visiting : Set(String)) : Array(String)
    deps_order = [] of String
    if visiting.includes?(package_name)
      raise "Dependency cycle detected involving #{package_name}"
    end
    visiting.add(package_name)
    if pkg = repo[package_name]?
      if deps = pkg["deps"]?.try(&.as_a?)
        deps.each do |dep|
          dep_name = dep.as_s
          unless installed.includes?(dep_name)
            sub_deps = resolve_deps(repo, dep_name, installed, visiting)
            deps_order.concat(sub_deps)
          end
        end
      end
      unless installed.includes?(package_name)
        deps_order << package_name
      end
    end
    visiting.delete(package_name)
    deps_order
  end

  def self.install(package_name : String)
    puts "Installing #{package_name}..."
    repo = JSON.parse(File.read("/usr/lib/HackerOS/hpm/repo.json")).as_h
    state = if File.exists?(STATE_PATH)
              JSON.parse(File.read(STATE_PATH)).as_h["packages"].as_h.transform_keys(&.to_s).transform_values(&.as_h.transform_keys(&.to_s))
            else
              {} of String => Hash(String, JSON::Any)
            end
    installed = Set.new(state.keys)
    begin
      to_install = resolve_deps(repo, package_name, installed, Set(String).new)
      to_install.each do |pkg_name|
        install_single(pkg_name, repo)
      end
      puts "#{package_name} and dependencies installed."
    rescue ex : Exception
      puts "Installation failed: #{ex.message}"
    end
  end

  def self.install_single(package_name : String, repo : Hash(String, JSON::Any))
    if pkg = repo[package_name]?
      pkg_url = pkg["url"].as_s
      version = pkg["version"].as_s
      expected_sha = pkg["sha256"]?.try(&.as_s?) || nil
      pkg_path = "#{STORE_PATH}#{package_name}/#{version}"
      current_link = "#{STORE_PATH}#{package_name}/current"
      temp_archive = "/tmp/#{package_name}.tar.gz"
      temp_extract = "#{pkg_path}.tmp"
      FileUtils.mkdir_p(temp_extract)
      download_file(pkg_url, temp_archive)
      if expected_sha
        computed_sha = compute_sha256(temp_archive)
        raise "SHA256 mismatch for #{package_name}" unless computed_sha == expected_sha
      end
      Process.run("tar", ["-xzf", temp_archive, "-C", temp_extract])
      # Call backend install
      output_io = IO::Memory.new
      error_io = IO::Memory.new
      status = Process.run(BACKEND_PATH, ["install", package_name, temp_extract, expected_sha || "none"], output: output_io, error: error_io)
      unless status.success?
        FileUtils.rm_rf(temp_extract)
        raise "Backend install failed: #{error_io.to_s}"
      end
      # Parse backend JSON
      json_output = JSON.parse(output_io.to_s)
      unless json_output["success"].as_bool
        FileUtils.rm_rf(temp_extract)
        raise "Backend reported failure"
      end
      FileUtils.mv(temp_extract, pkg_path)
      FileUtils.rm_rf(current_link) if File.symlink?(current_link)
      File.symlink(version, current_link)
      # Create wrappers
      manifest_path = "#{pkg_path}/manifest.json"
      manifest = JSON.parse(File.read(manifest_path)).as_h
      if bins = manifest["bins"]?.try(&.as_a?)
        bins.each do |bin|
          bin_name = bin.as_s
          wrapper_path = "/usr/bin/#{bin_name}"
          File.write(wrapper_path, <<-SCRIPT
          #!/bin/sh
          exec hpm-run #{package_name} #{bin_name} "$@"
          SCRIPT
          )
          File.chmod(wrapper_path, 0o755)
        end
      end
      # Update state
      state = if File.exists?(STATE_PATH)
                JSON.parse(File.read(STATE_PATH)).as_h
              else
                {"packages" => JSON::Any.new({} of String => JSON::Any)}
              end
      state["packages"].as_h[package_name] = JSON::Any.new({"version" => JSON::Any.new(version), "checksum" => JSON::Any.new(expected_sha || "none")})
      File.write(STATE_PATH, state.to_json)
    else
      puts "Package #{package_name} not found."
    end
  end

  def self.remove(package_name : String)
    puts "Removing #{package_name}..."
    state = if File.exists?(STATE_PATH)
              JSON.parse(File.read(STATE_PATH)).as_h["packages"].as_h.transform_keys(&.to_s).transform_values(&.as_h.transform_keys(&.to_s))
            else
              return puts "State not found."
            end
    if pkg_state = state[package_name]?
      version = pkg_state["version"].as_s
      installed_path = "#{STORE_PATH}#{package_name}/#{version}"
      current_link = "#{STORE_PATH}#{package_name}/current"
      # Call backend remove
      output_io = IO::Memory.new
      error_io = IO::Memory.new
      status = Process.run(BACKEND_PATH, ["remove", package_name, installed_path], output: output_io, error: error_io)
      unless status.success?
        raise "Backend remove failed: #{error_io.to_s}"
      end
      json_output = JSON.parse(output_io.to_s)
      unless json_output["success"].as_bool
        raise "Backend reported failure"
      end
      # Remove wrappers
      manifest_path = "#{installed_path}/manifest.json"
      if File.exists?(manifest_path)
        manifest = JSON.parse(File.read(manifest_path)).as_h
        if bins = manifest["bins"]?.try(&.as_a?)
          bins.each do |bin|
            FileUtils.rm("/usr/bin/#{bin.as_s}") rescue nil
          end
        end
      end
      FileUtils.rm_rf(installed_path)
      FileUtils.rm_rf(current_link) if File.symlink?(current_link)
      state.delete(package_name)
      full_state = {"packages" => state}
      File.write(STATE_PATH, full_state.to_json)
      puts "#{package_name} removed."
    else
      puts "Package #{package_name} not installed."
    end
  end

  def self.update
    puts "Updating installed packages..."
    repo = JSON.parse(File.read("/usr/lib/HackerOS/hpm/repo.json")).as_h
    state = if File.exists?(STATE_PATH)
              JSON.parse(File.read(STATE_PATH)).as_h["packages"].as_h.transform_keys(&.to_s).transform_values(&.as_h.transform_keys(&.to_s))
            else
              {} of String => Hash(String, JSON::Any)
            end
    state.each_key do |package_name|
      current_version = state[package_name]["version"].as_s
      if pkg = repo[package_name]?
        latest_version = pkg["version"].as_s
        if latest_version > current_version
          puts "Updating #{package_name} from #{current_version} to #{latest_version}"
          remove(package_name)
          install_single(package_name, repo)
        end
      end
    end
    puts "Updates complete."
  end

  def self.upgrade
    puts "Checking for HPM upgrade..."
    temp_version_file = "/tmp/hpm-version.hacker"
    download_file(VERSION_URL, temp_version_file)
    remote_version = File.read(temp_version_file).strip[1..-2] # Remove [ ]
    local_version = if File.exists?(LOCAL_VERSION_FILE)
                      JSON.parse(File.read(LOCAL_VERSION_FILE))["version"].as_s
                    else
                      "0.0"
                    end
    if remote_version > local_version
      puts "Upgrading HPM to #{remote_version}..."
      FileUtils.rm("/usr/bin/hpm") if File.exists?("/usr/bin/hpm")
      FileUtils.rm(BACKEND_PATH) if File.exists?(BACKEND_PATH)
      download_file("#{RELEASES_BASE}#{remote_version}/hpm", "/usr/bin/hpm")
      File.chmod("/usr/bin/hpm", 0o755)
      download_file("#{RELEASES_BASE}#{remote_version}/backend", BACKEND_PATH)
      File.chmod(BACKEND_PATH, 0o755)
      # Update local version
      File.write(LOCAL_VERSION_FILE, {"version" => remote_version}.to_json)
      puts "Upgrade complete."
    else
      puts "HPM is up to date."
    end
  end

  def self.main
    FileUtils.mkdir_p(STORE_PATH)
    FileUtils.mkdir_p(File.dirname(BACKEND_PATH))
    FileUtils.mkdir_p("/var/lib/hpm")
    command = ARGV[0]? || "help"
    case command
    when "refresh"
      refresh
    when "install"
      install(ARGV[1])
    when "remove"
      remove(ARGV[1])
    when "update"
      update
    when "upgrade"
      upgrade
    else
      puts "Usage: hpm [refresh|install <pkg>|remove <pkg>|update|upgrade]"
    end
  end
end

HPM.main
