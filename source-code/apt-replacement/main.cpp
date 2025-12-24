#include <iostream>
#include <string>
#include <fstream>
#include <vector>
#include <sstream>
#include <iomanip>
#include <cstdlib>
#include <unistd.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <ctime>
#include <curl/curl.h>
#include <sys/wait.h>

using namespace std;

const string CACHE_DIR = "/var/cache/hpm/";
const string LOG_DIR = "/tmp/hpm/logs/";

struct DownloadItem {
    string url;
    string filename;
    size_t size;
};

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

pair<int, string> run_command(const string& cmd, ofstream& log, const string& description, bool print_output = true, bool use_sudo = false) {
    string full_cmd = use_sudo ? "sudo " + cmd : cmd;
    if (!description.empty()) {
        cout << "\033[1;33m" << description << "\033[0m" << endl;
    }
    FILE* pipe = popen(full_cmd.c_str(), "r");
    if (!pipe) {
        log << "popen failed for: " << full_cmd << endl;
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
    int exit_code = WEXITSTATUS(status);
    log << "Command: " << full_cmd << "\nStdout/Stderr: " << result << "\nExit code: " << exit_code << endl;
    if (print_output) {
        string color = (exit_code == 0) ? "\033[1;32m" : "\033[1;31m";
        cout << color << result << "\033[0m" << endl;
    }
    return {exit_code, result};
}

bool is_package_installed(const string& package, ofstream& log) {
    auto [status, output] = run_command("dpkg-query -W -f='${Status}' " + package, log, "", false, false);
    return status == 0 && output.find("install ok installed") != string::npos;
}

vector<DownloadItem> parse_print_uris(const string& output) {
    vector<DownloadItem> downloads;
    stringstream ss(output);
    string line;
    while (getline(ss, line)) {
        if (line.empty() || line[0] != '\'') continue;
        stringstream ls(line);
        string url, filename;
        size_t size = 0;
        ls >> url >> filename >> size;
        url = url.substr(1, url.size() - 2);  // remove quotes
        downloads.push_back({url, filename, size});
    }
    return downloads;
}

struct ProgressData {
    double last_percent;
    time_t start_time;
    string filename;
};

int progress_callback(void *p, curl_off_t dltotal, curl_off_t dlnow, curl_off_t ultotal, curl_off_t ulnow) {
    ProgressData *prog = static_cast<ProgressData *>(p);
    if (dltotal <= 0) return 0;
    double percent = (double)dlnow / dltotal * 100.0;
    if (percent - prog->last_percent < 1.0) return 0;  // update every 1%

    time_t now = time(NULL);
    double elapsed = difftime(now, prog->start_time);
    if (elapsed == 0) elapsed = 1; // avoid division by zero
    double speed = dlnow / elapsed;
    double eta = (dltotal - dlnow) / speed;

    cout << "\r\033[1;36mDownloading " << prog->filename << " [";
    int bar_width = 50;
    int pos = static_cast<int>(percent / 100.0 * bar_width);
    for (int i = 0; i < bar_width; ++i) {
        if (i < pos) cout << "=";
        else if (i == pos) cout << ">";
        else cout << " ";
    }
    cout << "] " << fixed << setprecision(1) << percent << "% " 
         << (speed / 1024) << " KB/s eta " << eta << "s\033[0m" << flush;
    prog->last_percent = percent;
    return 0;
}

int download_file(const string& url, const string& path, ofstream& log) {
    CURL *curl = curl_easy_init();
    if (!curl) return 1;

    FILE *fp = fopen(path.c_str(), "wb");
    if (!fp) {
        curl_easy_cleanup(curl);
        return 1;
    }

    ProgressData prog = {0.0, time(NULL), path.substr(path.find_last_of('/') + 1)};

    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, NULL);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, fp);
    curl_easy_setopt(curl, CURLOPT_NOPROGRESS, 0L);
    curl_easy_setopt(curl, CURLOPT_XFERINFOFUNCTION, progress_callback);
    curl_easy_setopt(curl, CURLOPT_XFERINFODATA, &prog);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);

    CURLcode res = curl_easy_perform(curl);
    cout << endl;

    fclose(fp);
    curl_easy_cleanup(curl);

    if (res != CURLE_OK) {
        log << "Download failed for " << url << ": " << curl_easy_strerror(res) << endl;
        return 1;
    }
    return 0;
}

vector<string> download_packages(const vector<DownloadItem>& downloads, ofstream& log) {
    vector<string> paths;
    if (chdir(CACHE_DIR.c_str()) != 0) {
        log << "Failed to change to cache dir" << endl;
        return paths;
    }
    for (const auto& item : downloads) {
        string path = CACHE_DIR + item.filename;
        cout << "\033[1;33mStarting download: " << item.filename << "\033[0m" << endl;
        if (download_file(item.url, item.filename, log) == 0) {
            paths.push_back(path);
        } else {
            cout << "\033[1;31mDownload failed for " << item.filename << "\033[0m" << endl;
        }
    }
    return paths;
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        cout << "Usage: hpm [install|remove|clean|update|refresh] [package]" << endl;
        return 1;
    }

    string command = argv[1];
    string package = (argc > 2) ? argv[2] : "";

    create_dir(CACHE_DIR);
    create_dir(LOG_DIR);

    string log_path = LOG_DIR + "hpm_" + get_current_time() + ".log";
    ofstream log(log_path);
    if (!log.is_open()) {
        cout << "Failed to open log file" << endl;
        return 1;
    }

    log << "Starting hpm command: " << command << endl;

    if (command == "install") {
        if (package.empty()) {
            cout << "\033[1;31mNo package specified\033[0m" << endl;
            return 1;
        }
        if (is_package_installed(package, log)) {
            cout << "\033[1;32mPackage " << package << " is already installed.\033[0m" << endl;
            return 0;
        }

        run_command("apt update", log, "Refreshing package lists", true, true);

        auto [ustatus, uris] = run_command("apt-get --print-uris -y install " + package, log, "", false, false);
        if (ustatus != 0) return 1;

        auto downloads = parse_print_uris(uris);
        if (downloads.empty()) {
            cout << "\033[1;33mNo packages to download\033[0m" << endl;
            return 0;
        }

        auto deb_paths = download_packages(downloads, log);

        string install_cmd = "dpkg -i";
        for (const auto& p : deb_paths) {
            install_cmd += " " + p;
        }
        run_command(install_cmd, log, "Installing packages", true, true);

        for (const auto& p : deb_paths) {
            remove(p.c_str());
        }

        cout << "\033[1;32mSuccessfully installed " << package << "!\033[0m" << endl;

    } else if (command == "remove") {
        if (package.empty()) {
            cout << "\033[1;31mNo package specified\033[0m" << endl;
            return 1;
        }
        if (!is_package_installed(package, log)) {
            cout << "\033[1;31mPackage " << package << " is not installed.\033[0m" << endl;
            return 0;
        }

        run_command("dpkg --remove " + package, log, "Removing " + package, true, true);

        cout << "\033[1;31mSuccessfully removed " << package << "!\033[0m" << endl;

    } else if (command == "clean") {
        run_command("apt autoclean", log, "Running autoclean", true, true);
        run_command("apt autoremove", log, "Running autoremove", true, true);
        run_command("rm -f /var/cache/hpm/*.deb", log, "", false, false);
        cout << "\033[1;34mCleaned up packages!\033[0m" << endl;

    } else if (command == "update") {
        run_command("apt update", log, "Refreshing package lists", true, true);

        auto [sstatus, sim] = run_command("apt-get -s upgrade", log, "", false, false);
        if (sim.find("Inst ") == string::npos) {
            cout << "\033[1;32mAll packages are up to date.\033[0m" << endl;
            return 0;
        }

        auto [ustatus, uris] = run_command("apt-get --print-uris -y upgrade", log, "", false, false);
        if (ustatus != 0) return 1;

        auto downloads = parse_print_uris(uris);
        if (downloads.empty()) {
            cout << "\033[1;33mNo updates available\033[0m" << endl;
            return 0;
        }

        auto deb_paths = download_packages(downloads, log);

        string install_cmd = "dpkg -i";
        for (const auto& p : deb_paths) {
            install_cmd += " " + p;
        }
        run_command(install_cmd, log, "Upgrading packages", true, true);

        for (const auto& p : deb_paths) {
            remove(p.c_str());
        }

        cout << "\033[1;35mPackages updated!\033[0m" << endl;

    } else if (command == "refresh") {
        run_command("apt update", log, "Refreshing package lists", true, true);
        cout << "\033[1;36mPackage lists refreshed!\033[0m" << endl;

    } else {
        cout << "Unknown command" << endl;
        return 1;
    }

    log << "Command completed successfully" << endl;
    return 0;
}
