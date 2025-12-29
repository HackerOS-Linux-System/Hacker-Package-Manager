require "http/client"
require "json"
require "file_utils"
require "process"

module HPM
  VERSION = "0.3"

  STORE_PATH = "/usr/lib/HackerOS/hpm/store/"
  BACKEND_PATH = "#{ENV["HOME"]}/.hackeros/hpm/bin/backend"
  REPO_JSON_URL = "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/Community/repo.json"
  VERSION_URL = "https://raw.githubusercontent.com/HackerOS-Linux-System/Hacker-Package-Manager/main/Community/hpm-version.hacker"
  LOCAL_VERSION_FILE = "/usr/lib/HackerOS/hpm/version.json"
  RELEASES_BASE = "https://github.com/HackerOS-Linux-System/Hacker-Package-Manager/releases/download/v"

  def self.download_file(url : String, path : String)
    HTTP::Client.get(url) do |response|
      File.write(path, response.body_io)
    end
  end

  def self.refresh
    puts "Refreshing package index..."
    temp_path = "/tmp/repo.json"
    download_file(REPO_JSON_URL, temp_path)
    FileUtils.mv(temp_path, "/usr/lib/HackerOS/hpm/repo.json")
    puts "Package index refreshed."
  end

  def self.install(package_name : String)
    puts "Installing #{package_name}..."
    repo = JSON.parse(File.read("/usr/lib/HackerOS/hpm/repo.json")).as_h
    if pkg = repo[package_name]?
      pkg_url = pkg["url"].as_s
      version = pkg["version"].as_s
      install_path = "#{STORE_PATH}#{package_name}-#{version}"
      FileUtils.mkdir_p(install_path)
      temp_archive = "/tmp/#{package_name}.tar.gz"
      download_file(pkg_url, temp_archive)
      # Extract archive (assuming tar.gz)
      Process.run("tar", ["-xzf", temp_archive, "-C", install_path])
      # Call backend for isolation setup (bubblewrap-like)
      Process.run(BACKEND_PATH, ["install", package_name, install_path])
      puts "#{package_name} installed."
    else
      puts "Package #{package_name} not found."
    end
  end

  def self.remove(package_name : String)
    puts "Removing #{package_name}..."
    # Find installed version (simplified, assume we have a manifest)
    installed_path = Dir.glob("#{STORE_PATH}#{package_name}-*").first?
    if installed_path
      Process.run(BACKEND_PATH, ["remove", package_name, installed_path])
      FileUtils.rm_rf(installed_path)
      puts "#{package_name} removed."
    else
      puts "Package #{package_name} not installed."
    end
  end

  def self.update
    puts "Updating installed packages..."
    repo = JSON.parse(File.read("/usr/lib/HackerOS/hpm/repo.json")).as_h
    Dir.glob("#{STORE_PATH}*").each do |dir|
      if File.directory?(dir)
        package_name = File.basename(dir).split("-").first
        current_version = File.basename(dir).split("-").last
        if pkg = repo[package_name]?
          latest_version = pkg["version"].as_s
          if latest_version > current_version
            puts "Updating #{package_name} from #{current_version} to #{latest_version}"
            remove(package_name)
            install(package_name)
          end
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
      FileUtils.chmod("/usr/bin/hpm", 0o755)
      download_file("#{RELEASES_BASE}#{remote_version}/backend", BACKEND_PATH)
      FileUtils.chmod(BACKEND_PATH, 0o755)
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
