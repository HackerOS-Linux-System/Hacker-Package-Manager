require "http/client"
require "json"
require "file_utils"
require "process"
require "digest/sha256"

module HPM
  VERSION = "0.5"
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

  def self.compare_versions(a : String, b : String) : Int32
    parts_a = a.split(/[\.-]/)
    parts_b = b.split(/[\.-]/)
    max_len = [parts_a.size, parts_b.size].max
    (0...max_len).each do |i|
      return -1 if i >= parts_a.size
      return 1 if i >= parts_b.size
      pa = parts_a[i]
      pb = parts_b[i]
      ia = pa.to_i?
      ib = pb.to_i?
      if ia && ib
        cmp = ia <=> ib
        return cmp if cmp != 0
      else
        cmp = pa <=> pb
        return cmp if cmp != 0
      end
    end
    return 0
  end

  def self.satisfies(ver : String, req : String) : Bool
    return true if req.empty?
    if req.starts_with?(">=")
      req_ver = req[2..].strip
      compare_versions(ver, req_ver) >= 0
    elsif req.starts_with?(">")
      req_ver = req[1..].strip
      compare_versions(ver, req_ver) > 0
    elsif req.starts_with?("=")
      req_ver = req[1..].strip
      ver == req_ver
    else
      ver == req
    end
  end

  def self.choose_version(repo : Hash(String, JSON::Any), pkg_name : String, req : String, chosen : Hash(String, String))
    if chosen.has_key?(pkg_name)
      existing_ver = chosen[pkg_name]
      raise "Version conflict for #{pkg_name}: #{existing_ver} does not satisfy #{req}" unless satisfies(existing_ver, req)
      return
    end

    pkg_entry = repo[pkg_name]?
    vers = pkg_entry.try(&.[]?("versions")).try(&.as_a?) || [] of JSON::Any

    compatible = vers.select { |v| satisfies(v["version"].as_s, req) }
    raise "No version for #{pkg_name} satisfies #{req}" if compatible.empty?

    # Naprawione: Użycie sort z blokiem i pobranie ostatniego elementu (najwyższej wersji)
    max_v = compatible.sort { |a, b| compare_versions(a["version"].as_s, b["version"].as_s) }.last?

    chosen[pkg_name] = max_v.not_nil!["version"].as_s
  end

  def self.resolve_deps(repo : Hash(String, JSON::Any), pkg_name : String, req : String, chosen : Hash(String, String), visiting : Set(String), order : Array({String, String}))
    if visiting.includes?(pkg_name)
      raise "Dependency cycle detected involving #{pkg_name}"
    end
    visiting.add(pkg_name)
    choose_version(repo, pkg_name, req, chosen)
    ver = chosen[pkg_name]

    pkg_data = repo[pkg_name]?
    raise "Package #{pkg_name} not found in repo" unless pkg_data

    ver_obj = pkg_data["versions"].as_a.find { |v| v["version"].as_s == ver }.not_nil!
    deps = ver_obj["deps"]?.try(&.as_h?) || {} of String => JSON::Any
    deps.each do |dep, dep_req_obj|
      dep_req = dep_req_obj.as_s
      resolve_deps(repo, dep, dep_req, chosen, visiting, order)
    end
    order << {pkg_name, ver} unless order.any? { |p, v| p == pkg_name && v == ver }
    visiting.delete(pkg_name)
  end

  def self.install(package : String)
    puts "Installing #{package}..."
    repo_path = "/usr/lib/HackerOS/hpm/repo.json"
    return puts "Repo not found. Run 'hpm refresh' first." unless File.exists?(repo_path)

    repo = JSON.parse(File.read(repo_path)).as_h

    state = if File.exists?(STATE_PATH)
              JSON.parse(File.read(STATE_PATH)).as_h["packages"].as_h
            else
              {} of String => JSON::Any
            end

    installed = Hash(String, String).new
    state.each do |pkg, _|
      current_link = "#{STORE_PATH}#{pkg}/current"
      if File.symlink?(current_link)
        ver = File.readlink(current_link).split("/").last
        installed[pkg] = ver
      end
    end

    pkg_name, requested_ver = if package.includes?("@")
                                parts = package.split("@")
                                {parts[0], parts[1]}
                              else
                                {package, ""}
                              end

    req = requested_ver.empty? ? "" : requested_ver
    chosen = Hash(String, String).new
    visiting = Set(String).new
    order = [] of {String, String}

    begin
      resolve_deps(repo, pkg_name, req, chosen, visiting, order)
      order.each do |p, v|
        unless installed.has_key?(p) && satisfies(installed[p], chosen[p])
          install_single(p, v, repo)
        end
      end
      puts "#{package} and dependencies installed."
    rescue ex : Exception
      puts "Installation failed: #{ex.message}"
    end
  end

  def self.parse_info_hk(path : String) : Hash(String, Hash(String, JSON::Any))
    sections = {} of String => Hash(String, JSON::Any)
    current_section = ""
    last_key = ""
    return sections unless File.exists?("#{path}/info.hk")

    File.read_lines("#{path}/info.hk").each do |line|
      l = line.strip
      next if l.empty? || l.starts_with?("!")
      if l.starts_with?("[")
        end_idx = l.index("]")
        next unless end_idx
        current_section = l[1...end_idx]
        sections[current_section] = {} of String => JSON::Any
        next
      end
      if l.starts_with?("->")
        l = l[2..].strip
        if l.includes?("=>")
          key, value = l.split("=>", limit: 2).map(&.strip)
          sections[current_section][key] = JSON::Any.new(value)
          last_key = key
        else
          key = l
          sections[current_section][key] = JSON::Any.new(Hash(String, JSON::Any).new)
          last_key = key
        end
      elsif l.starts_with?("-->")
        l = l[3..].strip
        if l.includes?("=>")
          key, value = l.split("=>", limit: 2).map(&.strip)
          if sec = sections[current_section]?
            if item = sec[last_key]?
              item.as_h[key] = JSON::Any.new(value) if item.as_h?
            end
          end
        else
          key = l
          if sec = sections[current_section]?
            if item = sec[last_key]?
              if item.as_a?
                item.as_a << JSON::Any.new(key)
              else
                sec[last_key] = JSON::Any.new([JSON::Any.new(key)])
              end
            end
          end
        end
      end
    end
    sections
  end

  def self.install_single(package_name : String, version : String, repo : Hash(String, JSON::Any))
    if pkg = repo[package_name]?
      vers = pkg["versions"].as_a
      ver_obj = vers.find { |v| v["version"].as_s == version }
      raise "Version #{version} not found for #{package_name}" unless ver_obj

      pkg_url = ver_obj["url"].as_s
      expected_sha = ver_obj["sha256"]?.try(&.as_s?)
      pkg_path = "#{STORE_PATH}#{package_name}/#{version}"
      current_link = "#{STORE_PATH}#{package_name}/current"

      return puts "Already installed #{package_name}@#{version}" if Dir.exists?(pkg_path)

      temp_archive = "/tmp/#{package_name}-#{version}.hpm"
      temp_extract = "#{pkg_path}.tmp"

      FileUtils.rm_rf(temp_extract) if Dir.exists?(temp_extract)
      FileUtils.mkdir_p(temp_extract)

      puts "Downloading #{package_name}..."
      download_file(pkg_url, temp_archive)

      if expected_sha
        computed_sha = compute_sha256(temp_archive)
        if computed_sha != expected_sha
          FileUtils.rm_rf(temp_extract)
          raise "SHA256 mismatch for #{package_name}"
        end
      end

      status = Process.run("tar", ["-I", "zstd", "-xf", temp_archive, "-C", temp_extract])
      raise "Unpack failed" unless status.success?

      checksum = expected_sha || "none"
      status = Process.run(BACKEND_PATH, ["install", package_name, version, temp_extract, checksum])
      raise "Backend install failed" unless status.success?

      FileUtils.mv(temp_extract, pkg_path)
      FileUtils.mkdir_p(File.dirname(current_link))
      FileUtils.rm(current_link) if File.symlink?(current_link)
      File.symlink(version, current_link)

      manifest = parse_info_hk(pkg_path)
      if meta = manifest["metadata"]?
        if bins_any = meta["bins"]?
          bins = bins_any.as_a? ? bins_any.as_a.map(&.as_s) : [] of String
          bins.each do |bin|
            wrapper_path = "/usr/bin/#{bin}"
            File.write(wrapper_path, "#!/bin/sh\nexec #{BACKEND_PATH} run #{package_name} #{bin} \"$@\"\n")
            File.chmod(wrapper_path, 0o755)
          end
        end
      end
    else
      puts "Package #{package_name} not found."
    end
  end

  def self.remove(package : String)
    puts "Removing #{package}..."
    return puts "State not found." unless File.exists?(STATE_PATH)

    state = JSON.parse(File.read(STATE_PATH)).as_h
    packages = state["packages"].as_h

    pkg_name, version = if package.includes?("@")
                          parts = package.split("@")
                          {parts[0], parts[1]?}
                        else
                          {package, nil}
                        end

    return puts "Package #{pkg_name} not installed." unless packages.has_key?(pkg_name)

    vers_map = packages[pkg_name].as_h
    current_link = "#{STORE_PATH}#{pkg_name}/current"

    if version
      return puts "Version #{version} not installed." unless vers_map.has_key?(version)
      installed_path = "#{STORE_PATH}#{pkg_name}/#{version}"
      Process.run(BACKEND_PATH, ["remove", pkg_name, version, installed_path])
      FileUtils.rm_rf(installed_path)

      if File.symlink?(current_link) && File.readlink(current_link) == version
        FileUtils.rm(current_link)
      end
      vers_map.delete(version)
      packages.delete(pkg_name) if vers_map.empty?
    else
      vers_map.keys.each do |v|
        v_str = v.to_s
        installed_path = "#{STORE_PATH}#{pkg_name}/#{v_str}"
        Process.run(BACKEND_PATH, ["remove", pkg_name, v_str, installed_path])
        FileUtils.rm_rf(installed_path)
      end
      FileUtils.rm_rf("#{STORE_PATH}#{pkg_name}")
      packages.delete(pkg_name)
    end

    File.write(STATE_PATH, state.to_json)
    puts "#{package} removed."
  end

  def self.update
    puts "Updating installed packages..."
    repo_path = "/usr/lib/HackerOS/hpm/repo.json"
    return unless File.exists?(repo_path) && File.exists?(STATE_PATH)

    repo = JSON.parse(File.read(repo_path)).as_h
    state = JSON.parse(File.read(STATE_PATH)).as_h["packages"].as_h

    state.each_key do |pkg_name|
      current_link = "#{STORE_PATH}#{pkg_name}/current"
      next unless File.symlink?(current_link)

      current_ver = File.readlink(current_link).split("/").last rescue next
      if pkg = repo[pkg_name]?
        vers = pkg["versions"].as_a
        # Naprawione: Użycie sort.last? dla zachowania spójności
        latest_obj = vers.sort { |a, b| compare_versions(a["version"].as_s, b["version"].as_s) }.last?
        next unless latest_obj

        latest_ver = latest_obj["version"].as_s
        if compare_versions(latest_ver, current_ver) > 0
          puts "Updating #{pkg_name} from #{current_ver} to #{latest_ver}"
          remove("#{pkg_name}@#{current_ver}")
          install_single(pkg_name, latest_ver, repo)
        end
      end
    end
    puts "Updates complete."
  end

  def self.switch(pkg_name : String, version : String)
    puts "Switching #{pkg_name} to #{version}..."
    return puts "State not found." unless File.exists?(STATE_PATH)

    state = JSON.parse(File.read(STATE_PATH)).as_h["packages"].as_h
    return puts "Package #{pkg_name} not installed." unless state.has_key?(pkg_name)

    vers_map = state[pkg_name].as_h
    return puts "Version #{version} not installed." unless vers_map.has_key?(version)

    current_link = "#{STORE_PATH}#{pkg_name}/current"
    FileUtils.rm(current_link) if File.symlink?(current_link)
    File.symlink(version, current_link)
    puts "Switched #{pkg_name} to #{version}."
  end

  def self.upgrade
    puts "Checking for HPM upgrade..."
    temp_version_file = "/tmp/hpm-version.hacker"
    download_file(VERSION_URL, temp_version_file)

    remote_raw = File.read(temp_version_file).strip
    remote_version = remote_raw.gsub(/[\[\]]/, "")

    local_version = File.exists?(LOCAL_VERSION_FILE) ? JSON.parse(File.read(LOCAL_VERSION_FILE))["version"].as_s : "0.0"

    if compare_versions(remote_version, local_version) > 0
      puts "Upgrading HPM to #{remote_version}..."
      download_file("#{RELEASES_BASE}#{remote_version}/hpm", "/usr/bin/hpm")
      File.chmod("/usr/bin/hpm", 0o755)
      download_file("#{RELEASES_BASE}#{remote_version}/backend", BACKEND_PATH)
      File.chmod(BACKEND_PATH, 0o755)
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
    when "refresh" then refresh
    when "install" then ARGV[1]? ? install(ARGV[1]) : puts "Specify package."
    when "remove"  then ARGV[1]? ? remove(ARGV[1]) : puts "Specify package."
    when "update"  then update
    when "switch"  then (ARGV[1]? && ARGV[2]?) ? switch(ARGV[1], ARGV[2]) : puts "Usage: switch <pkg> <ver>"
    when "upgrade" then upgrade
    else
      puts "Usage: hpm [refresh|install|remove|update|switch|upgrade]"
    end
  end
end

HPM.main
