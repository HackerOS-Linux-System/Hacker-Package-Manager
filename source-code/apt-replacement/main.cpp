#include <iostream>
#include <string>
#include <fstream>
#include <vector>
#include <set>
#include <queue>
#include <sys/stat.h>
#include <sys/types.h>
#include <ctime>
#include <sstream>
#include <iomanip>
#include <cstdlib>
#include <unistd.h>
#include <apt-pkg/cachefile.h>
#include <apt-pkg/pkgcache.h>
#include <apt-pkg/init.h>
#include <apt-pkg/progress.h>

using namespace std;

const string CACHE_DIR = "/var/cache/hpm/";
const string LOG_DIR = "/tmp/hpm/logs/";

void create_dir(const string& dir) {
    mkdir(dir.c_str(), 0755);
}

string get_current_time() {
    time_t now = time(0);
    tm *ltm = localtime(&now);
    stringstream ss;
    ss << (1900 + ltm->tm_year) << setw(2) << setfill('0') << (1 + ltm->tm_mon)
       << setw(2) << setfill('0') << ltm->tm_mday << "_"
       << setw(2) << setfill('0') << ltm->tm_hour
       << setw(2) << setfill('0') << ltm->tm_min
       << setw(2) << setfill('0') << ltm->tm_sec;
    return ss.str();
}

pair<int, string> run_command(const string& cmd, ofstream& log, const string& description, bool print_output = true) {
    if (!description.empty()) {
        cout << "\033[1;33m" << description << "\033[0m" << endl;
    }
    FILE* pipe = popen(cmd.c_str(), "r");
    if (!pipe) {
        log << "popen failed for: " << cmd << endl;
        return {1, "popen failed"};
    }
    string result;
    char buffer[128];
    while (!feof(pipe)) {
        if (fgets(buffer, 128, pipe) != NULL) {
            result += buffer;
        }
    }
    int status = pclose(pipe);
    int exit_code = status / 256;
    log << "Command: " << cmd << "\nStdout/Stderr: " << result << "\nExit code: " << exit_code << endl;
    if (print_output) {
        string color = (exit_code == 0) ? "\033[1;32m" : "\033[1;31m";
        cout << color << result << "\033[0m" << endl;
    }
    return {exit_code, result};
}

bool is_package_installed(const pkgCache::PkgIterator& Pkg) {
    return !Pkg.CurrentVer().end();
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        cout << "Usage: hpm [install|remove|clean|update|refresh] [package]" << endl;
        return 1;
    }

    string command = argv[1];

    create_dir(CACHE_DIR);
    create_dir(LOG_DIR);

    string log_path = LOG_DIR + "hpm_" + get_current_time() + ".log";
    ofstream log(log_path);
    if (!log.is_open()) {
        cout << "Failed to open log file" << endl;
        return 1;
    }

    log << "Starting hpm command: " << command << endl;

    string package;
    if (argc > 2) package = argv[2];

    if (command == "install") {
        if (package.empty()) {
            cout << "\033[1;31mNo package specified\033[0m" << endl;
            return 1;
        }

        // Refresh package lists
        run_command("sudo apt update", log, "Refreshing package lists");

        // Initialize APT
        if (!pkgInitConfig(*_system)) {
            log << "pkgInitConfig failed" << endl;
            cout << "\033[1;31mInitialization failed\033[0m" << endl;
            return 1;
        }
        if (!pkgInitSystem(*_system, _system)) {
            log << "pkgInitSystem failed" << endl;
            cout << "\033[1;31mInitialization failed\033[0m" << endl;
            return 1;
        }

        OpTextProgress Prog;
        pkgCacheFile cachefile;
        if (!cachefile.Open(&Prog, false)) {
            log << "cachefile.Open failed" << endl;
            cout << "\033[1;31mFailed to open package cache\033[0m" << endl;
            return 1;
        }

        pkgDepCache depcache(cachefile);
        depcache.Init(&Prog);

        pkgCache *Cache = cachefile.GetPkgCache();

        pkgCache::PkgIterator targetPkg = Cache->FindPkg(package);
        if (targetPkg.end()) {
            cout << "\033[1;31mPackage " << package << " not found\033[0m" << endl;
            return 1;
        }

        if (is_package_installed(targetPkg)) {
            cout << "\033[1;32mPackage " << package << " is already installed.\033[0m" << endl;
            return 0;
        }

        // Collect all dependencies recursively (simple solver - to be replaced with Rust)
        set<string> all_packages;
        queue<string> to_process;
        to_process.push(package);

        while (!to_process.empty()) {
            string p = to_process.front();
            to_process.pop();

            if (all_packages.count(p)) continue;
            all_packages.insert(p);

            pkgCache::PkgIterator Pkg = Cache->FindPkg(p);
            if (Pkg.end()) continue;

            pkgCache::VerIterator Ver = depcache.GetCandidateVer(Pkg);
            if (Ver.end()) continue;

            for (pkgCache::DepIterator Dep = Ver.DependsList(); !Dep.end(); ++Dep) {
                if (Dep->Type == pkgCache::Dep::Depends || Dep->Type == pkgCache::Dep::PreDepends) {
                    string target = Dep.TargetPkg().Name();
                    to_process.push(target);
                }
            }
        }

        // Filter not installed
        vector<string> to_install;
        vector<string> deb_files;
        for (const auto& p : all_packages) {
            pkgCache::PkgIterator Pkg = Cache->FindPkg(p);
            if (!is_package_installed(Pkg)) {
                pkgCache::VerIterator Ver = depcache.GetCandidateVer(Pkg);
                if (!Ver.end()) {
                    string arch = Ver.Arch();
                    string ver_str = Ver.VerStr();
                    string deb = p + "_" + ver_str + "_" + arch + ".deb";
                    to_install.push_back(p);
                    deb_files.push_back(deb);
                }
            }
        }

        if (to_install.empty()) {
            cout << "\033[1;32mNothing to install\033[0m" << endl;
            return 0;
        }

        // Change to cache dir
        if (chdir(CACHE_DIR.c_str()) != 0) {
            log << "chdir failed" << endl;
            cout << "\033[1;31mFailed to change directory to cache\033[0m" << endl;
            return 1;
        }

        // Download
        for (const auto& p : to_install) {
            run_command("apt download " + p, log, "Downloading " + p);
        }

        // Install all
        string install_cmd = "sudo dpkg -i";
        for (const auto& f : deb_files) {
            install_cmd += " " + f;
        }
        auto [istatus, iout] = run_command(install_cmd, log, "Installing packages");

        if (istatus != 0) {
            return 1;
        }

        cout << "\033[1;32mSuccessfully installed " << package << "!\033[0m" << endl;

    } else if (command == "remove") {
        if (package.empty()) {
            cout << "\033[1;31mNo package specified\033[0m" << endl;
            return 1;
        }

        // Initialize APT for check
        if (!pkgInitConfig(*_system)) {
            log << "pkgInitConfig failed" << endl;
            return 1;
        }
        if (!pkgInitSystem(*_system, _system)) {
            log << "pkgInitSystem failed" << endl;
            return 1;
        }

        OpTextProgress Prog;
        pkgCacheFile cachefile;
        if (!cachefile.Open(&Prog, false)) {
            log << "cachefile.Open failed" << endl;
            return 1;
        }

        pkgDepCache depcache(cachefile);
        depcache.Init(&Prog);

        pkgCache *Cache = cachefile.GetPkgCache();

        pkgCache::PkgIterator Pkg = Cache->FindPkg(package);
        if (Pkg.end()) {
            cout << "\033[1;31mPackage " << package << " not found\033[0m" << endl;
            return 1;
        }

        if (!is_package_installed(Pkg)) {
            cout << "\033[1;31mPackage " << package << " is not installed.\033[0m" << endl;
            return 0;
        }

        run_command("sudo dpkg --remove " + package, log, "Removing " + package);

        cout << "\033[1;31mSuccessfully removed " << package << "!\033[0m" << endl;

    } else if (command == "clean") {
        run_command("sudo apt autoclean", log, "Running autoclean");
        run_command("sudo apt autoremove", log, "Running autoremove");
        // Clean hpm cache
        run_command("rm -f " + CACHE_DIR + "*.deb", log, "Cleaning hpm cache", false);
        cout << "\033[1;34mCleaned up packages!\033[0m" << endl;

    } else if (command == "update") {
        run_command("sudo apt update", log, "Refreshing package lists");
        run_command("sudo apt upgrade -y", log, "Upgrading packages");
        cout << "\033[1;35mPackages updated!\033[0m" << endl;

    } else if (command == "refresh") {
        run_command("sudo apt update", log, "Refreshing package lists");
        cout << "\033[1;36mPackage lists refreshed!\033[0m" << endl;

    } else {
        cout << "Unknown command" << endl;
        return 1;
    }

    log << "Command completed successfully" << endl;
    return 0;
}
