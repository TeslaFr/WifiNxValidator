# WifiNx Validator

**English** | [Français](README-FR.md)

Native Rust application for Windows, Linux, and macOS that runs local DNS and HTTP responders for Nintendo connection checks, inspired by [Offline-WAN-Emulator](https://github.com/TheNxtBigThing8/Offline-WAN-Emulator). It provides shared service controls, a searchable request history, and a notification area icon.

The application emulates local connection checks. It does not configure IP addresses, DHCP, routing, connection sharing, or firewall rules. A successful local connection test does not prove access to Nintendo services or guarantee that update prompts will disappear. Switch and Switch 2 behavior must be checked on your own hardware.

## Getting started

The project version is **0.1.0**. Download an archive from GitHub Releases and extract it before launching. Targets are Windows x64, Linux x64 (Ubuntu 22.04), Linux ARM64/aarch64 (Ubuntu 24.04), macOS Intel x86_64, and Apple Silicon aarch64 (macOS 11 minimum). Archives include both READMEs and notices; `SHA256SUMS` provides their checksums. Locally packaged archives are placed in `dist/`, which is ignored by Git.

- **Windows:** the release executable requests administrator rights through UAC to use ports 53 and 80. Allow incoming traffic on the console's local network when prompted by the firewall. The MSVC C/C++ runtime is statically linked. Graphics use Direct3D 12 through wgpu, preferring the WARP software adapter when supported and falling back to hardware.
- **Linux:** GTK 3, AppIndicator/StatusNotifier support, `gdbus`, Polkit (`pkexec`), and `setcap` are required. On first launch, Polkit authorizes only `setcap`; the GUI remains in your user session. A graphical Polkit agent must be running. The executable's filesystem must support Linux capabilities and must not be mounted with `nosuid`. Rebuilding, copying, or extracting the executable can require authorization again. Linux uses OpenGL.

- **macOS:** rendering uses Metal through wgpu, and the menu bar icon is created on the main thread. The `.app` bundle is ad hoc signed, without Apple notarization; approval in **System Settings → Privacy & Security** may be required. Normally keep **Listen address** at `0.0.0.0`: the current Apple kernel permits reserved-port listening on this wildcard address, while a specific IP address may require privileges. Automatic macOS elevation is not provided.

The interface starts in French when the detected language is French, and English otherwise, using the Windows UI language, Linux locale, or macOS user preferences through CoreFoundation. The button to the left of **Hide / Masquer** shows the target language: **EN** in the French interface and **FR** in the English interface. It switches the interface and tray menu immediately without restarting DNS/HTTP. Your manual choice is saved.

## Configure the application and Nintendo Switch

### 1. Prepare the network

Connect the PC and console to the **same local network**. Avoid a guest network that isolates devices. Find the PC's local **IPv4 address** on that interface and the network's **subnet mask**; for example, `192.168.1.27` and `255.255.255.0`.

All addresses below are examples to adapt. A manually assigned console address must be unused and reserved for this purpose, or otherwise excluded from the router's DHCP assignments.

### 2. Configure the application

Launch WifiNx Validator and accept the authorization request if shown.

| Field | Value |
| --- | --- |
| **Listen address** | `0.0.0.0` listens on all IPv4 interfaces. To avoid conflicts or select one interface, use the PC's local address, such as `192.168.1.27`. |
| **Advertised PC IP** | The PC's actual local IPv4 address, such as `192.168.1.27`. Do not use `0.0.0.0` here. |
| **DNS port / HTTP port** | Fixed, read-only values: **53 / 80**. |

Click **Start DNS + HTTP** and check that both services are active. The PC firewall must allow the console to reach **UDP 53 and TCP 53/80**. Starting the services saves the settings; services remain stopped on the next application launch. **Save settings** also saves your configuration without starting them.

### 3. Configure the console

Open **System Settings → Internet → Internet Settings**, then create a connection using **Manual Setup**. Labels can vary by console and system version.

| Wi-Fi setting | Value |
| --- | --- |
| **Connection name** | `DnsProtect`, or any name you prefer. |
| **SSID** | Your exact Wi-Fi network name. |
| **Security** | Your network's security type, usually **WPA2-PSK (AES)**. |
| **Password** | Your Wi-Fi password. |

Set **IP Address Settings** to **Manual**. This example assumes a `192.168.1.x` network with the PC at `192.168.1.27`.

| IP setting | Value |
| --- | --- |
| **IP address** | A free address in the same subnet, such as `192.168.1.200`, if available and reserved for this use. |
| **Subnet mask** | Your network's actual mask, for example `255.255.255.0`. |
| **Gateway** | To prevent normal IPv4 routing to the Internet, an **unused address in the same subnet**, such as `192.168.1.254` only if no device uses it. |

`192.168.253.254` is suitable only for a matching subnet, such as `192.168.253.x` with mask `255.255.255.0`; it is not a universal gateway value. Devices in the same subnet can communicate locally without a working gateway; see [RFC 1122, section 3.3.1](https://www.rfc-editor.org/rfc/rfc1122.html#section-3.3.1).

Set **DNS Settings** to **Manual**:

| DNS setting | Value |
| --- | --- |
| **Primary DNS** | The PC's local IPv4 address, for example `192.168.1.27`. |
| **Secondary DNS** | `000.000.000.000`. |

Do not add a public secondary DNS server: it would offer another resolution path. Save the connection and run its connection test. Check that the console's DNS and HTTP requests appear in the application's **History**.

Nintendo's official instructions cover [manual Wi-Fi setup](https://www.nintendo.com/fr-fr/Assistance/Depannage/Comment-configurer-une-connexion-Internet-sans-fil-manuellement-sur-Nintendo-Switch-1494762.html) and [manual DNS settings](https://www.nintendo.com/fr-fr/Assistance/Depannage/Comment-saisir-les-parametres-du-DNS-manuellement-sur-la-Nintendo-Switch-2-ou-la-Nintendo-Switch-1496747.html). Use the values described above for this setup.

### 4. Understand the isolation limits

The PC acts as a local DNS responder that redirects the supported Nintendo test domains to itself, then answers their HTTP requests. Other domains are refused without forwarding queries to an Internet DNS server.

**DNS alone is not a firewall and does not guarantee Internet isolation.** It does not block connections to known IP addresses or other network paths. An unused gateway prevents normal IPv4 routing but does not guarantee isolation for every protocol.

For an offline network, use a dedicated network with no Internet access, or block the console's Internet access at the router for **both IPv4 and IPv6**. Do not share the PC's Internet connection with this network.

### 5. Can IP settings remain automatic?

Yes: **Automatic / DHCP** can assign the console's address while DNS remains manually set to the PC. However, DHCP usually also supplies the router's **real gateway**. If that gateway has Internet access, DNS filtering cannot guarantee isolation. For the offline setup above, keep the manual IP settings or enforce isolation on the network itself.

## Window, history, and settings

**Stop** stops both services. **History** displays UTC timestamps, client addresses, service, domain/path, request type, and result. **Filter** searches these fields; **Clear** clears the history. Up to **5,000 entries** are retained in memory and disappear when the application quits.

The window has no system title bar. Drag anywhere on the top strip except the three **language**, **Hide**, and **Quit** buttons to move it. **Hide** keeps services running in the notification area; the tray menu can open the window, start/stop the services, or quit completely. **Quit** stops services and exits.

Linux uses AppIndicator/StatusNotifier with a dedicated GTK loop. GNOME may require an AppIndicator extension; KDE Plasma normally supports it. If a working tray is unavailable, the application keeps the window visible and explains the problem. `gdbus` must be in `PATH`.

Settings are stored in `settings.json` under the user directory supplied by `directories::ProjectDirs`, usually `%APPDATA%\WifiNx\WifiNxValidator\config` on Windows and `~/.config/wifinxvalidator` on Linux. Ports are fixed and old saved port values are ignored.

## Protocol behavior

DNS supports UDP and TCP. A queries for these domains and their subdomains return the advertised IPv4 address; AAAA and unsupported record types receive an empty answer. Other domains are refused, and nothing is forwarded to the Internet.

- `ctest.p01.ctest.srv.nintendo.net`
- `ctest-ipv6.p01.ctest.srv.nintendo.net`
- `ctest.cdn.nintendo.net`
- `conntest.nintendowifi.net`

HTTP accepts the exact hostnames above. `GET /` returns `200 OK`, `X-Organization: Nintendo`, and body `ok`. `HEAD /` returns the same headers without a body. Other paths or hosts return 404; unsupported methods return 405. Connections are limited to 64 clients, requests to three seconds, and HTTP headers to 8 KiB.

## Troubleshooting and verification

A permission error is different from an occupied port. On Windows, use the release build with UAC; on Linux, check Polkit authorization or `CAP_NET_BIND_SERVICE`. On macOS, if listening on a specific IP fails with `permission denied`, return to `0.0.0.0` if listening on all interfaces suits your network. This reserved-port behavior is implemented in the [Apple XNU source](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/netinet/in_pcb.c).

If a port is occupied, select the PC's specific LAN address in **Listen address** instead of `0.0.0.0`. For example, libvirt's DNS on `192.168.122.1:53` can conflict with listening on every interface. Inspect Linux listeners with:

```sh
ss -lunp 'sport = :53'
ss -ltnp '( sport = :53 or sport = :80 )'
ps -eo user,pid,args
```

Only stop another service if you understand its purpose. For example, `sudo virsh -c qemu:///system net-destroy default` stops libvirt's default virtual network and disconnects its guests; stopping the daemon alone may leave its `dnsmasq` process running.

Windows startup logs are written to `%LOCALAPPDATA%\WifiNxValidator\startup.log`, or a temporary `wifinx-startup-<pid>.log` if that directory is unavailable. They include startup steps, the graphics adapter, and errors. Rust errors and panics can display a native dialog even before the GUI opens. Failures in the Windows loader before the application starts cannot be recorded there. An old build requiring `VCRUNTIME140.dll` can be replaced with a current release build or supplied with the [official Visual C++ x64 Redistributable](https://aka.ms/vc14/vc_redist.x64.exe).

With services running, adapt the address and check:

```sh
nslookup ctest.cdn.nintendo.net 192.168.1.27
curl --noproxy '*' --resolve ctest.cdn.nintendo.net:80:192.168.1.27 -i http://ctest.cdn.nintendo.net/
```

Expect the advertised IP from DNS, then HTTP status 200, the Nintendo header, and `ok`. In PowerShell, use `curl.exe` to avoid the `curl` alias. If the console's requests are absent from History, check the selected network interface, firewall, console DNS settings, and client isolation on the access point.

## Build and test

Install stable Rust and Cargo. The first build needs Internet access to download dependencies; the compiled application works offline. The DNS/HTTP core uses only Rust's standard library.

### Windows

Install the MSVC Rust toolchain and Visual Studio C++ build tools (**Desktop development with C++**):

```powershell
cargo build --release
.\target\release\wifinx-validator.exe
```

Keep `.cargo/config.toml`, `build.rs`, and `assets/windows.manifest` with the sources: release MSVC builds statically link the C/C++ runtime and embed the UAC `requireAdministrator` manifest. Development and test executables do not request UAC elevation. Release builds have no console window.

### Linux

On Debian/Ubuntu:

```sh
sudo apt install build-essential pkg-config libgtk-3-dev libayatana-appindicator3-dev libglib2.0-bin pkexec libcap2-bin
cargo build --release
./target/release/wifinx-validator
```

On Arch Linux, install `base-devel`, `pkgconf`, `gtk3`, `libayatana-appindicator`, `glib2`, `polkit`, and `libcap`. Package names vary by distribution. Linux binaries are dynamically linked; build locally if an archive's glibc or system library requirements do not match your system.

To grant the privileged-port capability manually:

```sh
sudo setcap cap_net_bind_service=+ep target/release/wifinx-validator
./target/release/wifinx-validator
```

### macOS

Install the Xcode Command Line Tools and stable Rust, then build on the target Mac:

```sh
xcode-select --install
cargo build --release
./target/release/wifinx-validator
```

Release archives provide the ad hoc signed `.app` bundle; a local Cargo build produces the executable. macOS 11 or later is required.

### Checks

```sh
cargo fmt --check
cargo test --no-default-features
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

For core checks without crates.io access:

```sh
./scripts/test-core.sh          # all core tests, including loopback sockets
./scripts/test-core.sh --pure   # tests without opening sockets
```

`scripts/check-windows-build.ps1` checks a Windows release build's imports and administrator manifest with the Visual Studio tools and Windows SDK installed. The `.github/workflows/ci.yml` workflow builds all five native targets and publishes their archives and `SHA256SUMS` in a GitHub Release only when a strict `vX.X.X` tag matching the `Cargo.toml` version is pushed. Branch and pull request events do not trigger builds or release publication. Compilation and automated checks do not replace testing the GUI, tray, graphics drivers, or console on target hardware.

To publish version `0.1.0` after reviewing the code and Cargo version, a maintainer can run (replace `TeslaFr` with your remote name if needed):

```sh
git tag v0.1.0
git push TeslaFr v0.1.0
```

## License and attribution

MIT; see [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). DNS/HTTP behavior is adapted from [setup_pi_wan.sh](https://github.com/TheNxtBigThing8/Offline-WAN-Emulator/blob/main/setup_pi_wan.sh), copyright 2026 TheNxtBigThing8, licensed under MIT.

GUI libraries: [eframe](https://github.com/emilk/egui) and [tray-icon](https://github.com/tauri-apps/tray-icon). Permission/build references: [Rust static C runtimes](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes), [Microsoft UAC manifests](https://learn.microsoft.com/en-us/windows/win32/secbp/running-with-administrator-privileges/), and [pkexec/Polkit](https://www.freedesktop.org/software/polkit/docs/master/pkexec.1.html).
