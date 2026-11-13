# WifiNx Validator

[English](README.md) | **Français**

Application Rust native pour Windows, Linux et macOS : répondeurs DNS et HTTP locaux pour les tests de connexion Nintendo, inspirés de [Offline-WAN-Emulator](https://github.com/TheNxtBigThing8/Offline-WAN-Emulator). Elle propose un démarrage/arrêt commun des services, un historique filtrable et une icône dans la zone de notification.

L'application reproduit des tests de connexion locaux. Elle ne configure ni adresses IP, DHCP, routage, partage de connexion ou pare-feu. Un test local réussi ne prouve pas l'accès aux services Nintendo et ne garantit pas l'absence de demandes de mise à jour. Le comportement Switch et Switch 2 doit être vérifié sur votre matériel.

## Prise en main

La version du projet est **0.1.0**. Téléchargez une archive depuis les GitHub Releases et extrayez-la avant de lancer l’application. Les cibles sont Windows x64, Linux x64 (Ubuntu 22.04), Linux ARM64/aarch64 (Ubuntu 24.04), macOS Intel x86_64 et Apple Silicon aarch64 (macOS 11 minimum). Les archives contiennent les deux README et les notices ; `SHA256SUMS` fournit leurs empreintes. Les archives préparées localement sont placées dans `dist/`, ignoré par Git.

- **Windows :** l'exécutable release demande les droits administrateur via UAC pour utiliser les ports 53 et 80. Autorisez les connexions entrantes sur le réseau local de la console si le pare-feu le demande. Le runtime C/C++ MSVC est lié statiquement. Le rendu utilise Direct3D 12 via wgpu, en privilégiant l'adaptateur logiciel WARP lorsqu'il est compatible, puis un adaptateur matériel.
- **Linux :** GTK 3, le support AppIndicator/StatusNotifier, `gdbus`, Polkit (`pkexec`) et `setcap` sont nécessaires. Au premier lancement, Polkit autorise uniquement `setcap` ; l'interface reste dans votre session utilisateur. Un agent Polkit graphique doit être actif. Le système de fichiers doit accepter les capacités Linux et ne pas être monté avec `nosuid`. Une compilation, copie ou extraction du binaire peut nécessiter une nouvelle autorisation. Linux utilise OpenGL.

- **macOS :** le rendu utilise Metal via wgpu et l’icône de barre des menus est créée sur le thread principal. Le bundle `.app` est signé ad hoc, sans notarisation Apple ; une autorisation dans **Réglages Système → Confidentialité et sécurité** peut être nécessaire. Gardez normalement **Adresse d’écoute** sur `0.0.0.0` : le noyau Apple permet actuellement l’écoute des ports réservés sur cette adresse générique, mais une adresse IP précise peut nécessiter des privilèges. Aucune élévation macOS automatique n’est fournie.

L’interface démarre en français si la langue détectée est française, en anglais sinon : langue d’interface Windows, locale Linux ou préférences utilisateur macOS via CoreFoundation. Le bouton à gauche de **Masquer / Hide** affiche la langue cible : **EN** dans l’interface française et **FR** dans l’interface anglaise. Il traduit immédiatement l’interface et le menu de notification sans redémarrer DNS/HTTP. Votre choix manuel est enregistré.

## Configurer l'application et la Nintendo Switch

### 1. Préparer le réseau

Connectez le PC et la console au **même réseau local**. Évitez un réseau invité qui isole les appareils. Relevez l'**adresse IPv4 locale du PC** sur cette interface et le **masque de sous-réseau** : par exemple `192.168.1.27` et `255.255.255.0`.

Toutes les adresses ci-dessous sont des exemples à adapter. L'adresse manuelle de la console doit être libre et réservée pour cet usage, ou exclue des attributions DHCP du routeur.

### 2. Configurer l'application

Lancez WifiNx Validator et acceptez la demande d'autorisation si elle apparaît.

| Champ | Valeur |
| --- | --- |
| **Adresse d'écoute** | `0.0.0.0` écoute sur toutes les interfaces IPv4. En cas de conflit ou pour choisir une interface, utilisez l'adresse locale du PC, par exemple `192.168.1.27`. |
| **IP du PC annoncée** | L'adresse IPv4 locale réelle du PC, par exemple `192.168.1.27`. Ne mettez pas `0.0.0.0` ici. |
| **Port DNS / Port HTTP** | Valeurs fixes en lecture seule : **53 / 80**. |

Cliquez sur **Activer DNS + HTTP** et vérifiez que les deux services sont actifs. Le pare-feu du PC doit autoriser la console sur **UDP 53 et TCP 53/80**. Le démarrage enregistre les paramètres ; les services restent arrêtés au prochain lancement de l'application. **Enregistrer les paramètres** permet aussi de sauvegarder la configuration sans démarrer les services.

### 3. Configurer la console

Ouvrez **Paramètres de la console → Internet → Paramètres Internet**, puis créez une connexion avec **Configuration manuelle** ou **Paramétrage manuel**. Les libellés peuvent varier selon la console et la version du système.

| Paramètre Wi-Fi | Valeur |
| --- | --- |
| **Nom de la connexion** | `DnsProtect`, ou le nom de votre choix. |
| **SSID** | Le nom exact de votre réseau Wi-Fi. |
| **Sécurité** | Le type utilisé par votre réseau, généralement **WPA2-PSK (AES)**. |
| **Mot de passe** | Le mot de passe de votre Wi-Fi. |

Réglez **Paramétrage de l'adresse IP** sur **Manuel**. Cet exemple correspond à un réseau `192.168.1.x`, avec le PC à `192.168.1.27`.

| Paramètre IP | Valeur |
| --- | --- |
| **Adresse IP** | Une adresse libre du même sous-réseau, par exemple `192.168.1.200`, si elle est disponible et réservée pour cet usage. |
| **Masque de sous-réseau** | Celui de votre réseau, par exemple `255.255.255.0`. |
| **Passerelle** | Pour empêcher le routage IPv4 normal vers Internet, une **adresse inutilisée du même sous-réseau**, par exemple `192.168.1.254` uniquement si aucun appareil ne l'utilise. |

`192.168.253.254` convient seulement à un sous-réseau correspondant, par exemple `192.168.253.x` avec le masque `255.255.255.0` ; ce n'est pas une valeur universelle. Des appareils du même sous-réseau communiquent localement sans passerelle fonctionnelle ; voir la [RFC 1122, section 3.3.1](https://www.rfc-editor.org/rfc/rfc1122.html#section-3.3.1).

Réglez **Paramétrage du DNS** sur **Manuel** :

| Paramètre DNS | Valeur |
| --- | --- |
| **DNS primaire** | L'adresse IPv4 locale du PC, par exemple `192.168.1.27`. |
| **DNS secondaire** | `000.000.000.000`. |

N'ajoutez pas de DNS public secondaire : il fournirait une autre possibilité de résolution. Enregistrez la connexion, puis lancez son test de connexion. Vérifiez que les requêtes DNS et HTTP de la console apparaissent dans l'**Historique** de l'application.

Les instructions officielles Nintendo expliquent la [connexion Wi-Fi manuelle](https://www.nintendo.com/fr-fr/Assistance/Depannage/Comment-configurer-une-connexion-Internet-sans-fil-manuellement-sur-Nintendo-Switch-1494762.html) et le [paramétrage manuel du DNS](https://www.nintendo.com/fr-fr/Assistance/Depannage/Comment-saisir-les-parametres-du-DNS-manuellement-sur-la-Nintendo-Switch-2-ou-la-Nintendo-Switch-1496747.html). Utilisez les valeurs ci-dessus pour cette configuration.

### 4. Comprendre les limites d'isolation

Le PC agit comme un DNS local « menteur » : les domaines de test Nintendo pris en charge renvoient vers le PC, qui répond aussi à leurs requêtes HTTP. Les autres domaines sont refusés sans transmettre les requêtes à un DNS Internet.

**Le DNS seul n'est pas un pare-feu et ne garantit pas l'absence d'accès à Internet.** Il ne bloque pas les connexions vers des adresses IP connues ni les autres chemins réseau. Une passerelle inutilisée empêche le routage IPv4 normal, mais ne garantit pas l'isolation de tous les protocoles.

Pour un réseau hors ligne, utilisez un réseau dédié sans accès Internet, ou bloquez l'accès Internet de la console sur le routeur en **IPv4 et en IPv6**. Ne partagez pas la connexion Internet du PC vers ce réseau.

### 5. Peut-on laisser l'adresse IP en automatique ?

Oui : **Automatique / DHCP** peut attribuer l'adresse de la console tout en conservant le DNS manuel vers le PC. Cependant, DHCP fournit généralement aussi la **vraie passerelle du routeur**. Si celle-ci donne accès à Internet, le filtrage DNS ne garantit pas l'isolation. Pour la configuration hors ligne décrite ci-dessus, conservez les paramètres IP manuels ou imposez l'isolation directement sur le réseau.

## Fenêtre, historique et paramètres

**Arrêter** arrête les deux services. L'**Historique** affiche date/heure UTC, client, service, domaine/chemin, type de requête et résultat. Le **Filtre** recherche dans ces champs ; **Effacer** vide l'historique. Les **5 000 dernières entrées** sont conservées en mémoire et disparaissent à la fermeture de l'application.

La fenêtre n'a pas de barre de titre système. Déplacez-la en faisant glisser toute zone de la bande supérieure sauf les trois boutons **langue**, **Masquer** et **Quitter**. **Masquer** conserve les services actifs dans la zone de notification ; le menu de l'icône permet d'ouvrir la fenêtre, de démarrer/arrêter les services ou de quitter complètement. **Quitter** arrête les services et termine l'application.

Linux utilise AppIndicator/StatusNotifier avec une boucle GTK dédiée. GNOME peut nécessiter une extension AppIndicator ; KDE Plasma le prend normalement en charge. Si la zone de notification n'est pas disponible, l'application conserve la fenêtre visible et explique le problème. `gdbus` doit être dans le `PATH`.

Les paramètres sont enregistrés dans `settings.json`, sous le répertoire utilisateur fourni par `directories::ProjectDirs`, habituellement `%APPDATA%\WifiNx\WifiNxValidator\config` sur Windows et `~/.config/wifinxvalidator` sur Linux. Les ports sont fixes ; les anciennes valeurs enregistrées sont ignorées.

## Comportement des protocoles

Le DNS utilise UDP et TCP. Les requêtes A pour les domaines suivants et leurs sous-domaines reçoivent l'IPv4 annoncée ; AAAA et les types non disponibles reçoivent une réponse vide. Les autres domaines sont refusés et aucune requête n'est transmise à Internet.

- `ctest.p01.ctest.srv.nintendo.net`
- `ctest-ipv6.p01.ctest.srv.nintendo.net`
- `ctest.cdn.nintendo.net`
- `conntest.nintendowifi.net`

Le HTTP accepte les noms exacts ci-dessus. `GET /` répond `200 OK`, `X-Organization: Nintendo`, corps `ok`. `HEAD /` renvoie les mêmes en-têtes sans corps. Les autres chemins ou hôtes répondent 404 ; les méthodes non prises en charge répondent 405. Les connexions sont limitées à 64 clients, les requêtes à trois secondes et les en-têtes HTTP à 8 Kio.

## Diagnostic et vérification

Une erreur de permission est distincte d’un port déjà occupé. Sous Windows, utilisez la release avec UAC ; sous Linux, vérifiez l’autorisation Polkit ou la capacité `CAP_NET_BIND_SERVICE`. Sous macOS, si une écoute sur une IP précise échoue avec `permission denied`, revenez à `0.0.0.0` si une écoute sur toutes les interfaces convient à votre réseau. Ce comportement des ports réservés est décrit dans le [code Apple XNU](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/netinet/in_pcb.c).

Si un port est occupé, choisissez l'adresse LAN précise du PC dans **Adresse d'écoute** au lieu de `0.0.0.0`. Le DNS de libvirt sur `192.168.122.1:53`, par exemple, peut entrer en conflit avec une écoute sur toutes les interfaces. Pour inspecter les écoutes Linux :

```sh
ss -lunp 'sport = :53'
ss -ltnp '( sport = :53 or sport = :80 )'
ps -eo user,pid,args
```

N'arrêtez un autre service que si vous connaissez son rôle. Par exemple, `sudo virsh -c qemu:///system net-destroy default` arrête le réseau virtuel libvirt par défaut et déconnecte ses machines virtuelles ; arrêter le démon seul peut laisser son processus `dnsmasq` actif.

Le journal de démarrage Windows est `%LOCALAPPDATA%\WifiNxValidator\startup.log`, ou un fichier temporaire `wifinx-startup-<pid>.log` si ce répertoire est inaccessible. Il contient les étapes de démarrage, l'adaptateur graphique et les erreurs. Les erreurs et paniques Rust peuvent afficher une boîte de dialogue native avant même l'ouverture de l'interface. Un échec du chargeur Windows avant le démarrage de l'application ne peut pas y être enregistré. Un ancien binaire réclamant `VCRUNTIME140.dll` peut être remplacé par une compilation release actuelle ou accompagné du [Visual C++ Redistributable x64 officiel](https://aka.ms/vc14/vc_redist.x64.exe).

Avec les services actifs, adaptez l'adresse puis vérifiez :

```sh
nslookup ctest.cdn.nintendo.net 192.168.1.27
curl --noproxy '*' --resolve ctest.cdn.nintendo.net:80:192.168.1.27 -i http://ctest.cdn.nintendo.net/
```

Le DNS doit renvoyer l'IP annoncée, puis HTTP le statut 200, l'en-tête Nintendo et `ok`. Dans PowerShell, utilisez `curl.exe` pour éviter l'alias `curl`. Si les requêtes de la console n'apparaissent pas dans l'historique, vérifiez l'interface réseau, le pare-feu, son DNS et l'isolation des clients sur le point d'accès.

## Compiler et tester

Installez Rust stable et Cargo. La première compilation nécessite Internet pour télécharger les dépendances ; l'application compilée fonctionne hors ligne. Le cœur DNS/HTTP utilise uniquement la bibliothèque standard Rust.

### Windows

Installez la chaîne Rust MSVC et les outils C++ Visual Studio (**Développement Desktop en C++**) :

```powershell
cargo build --release
.\target\release\wifinx-validator.exe
```

Conservez `.cargo/config.toml`, `build.rs` et `assets/windows.manifest` avec les sources : les compilations release MSVC lient statiquement le runtime C/C++ et intègrent le manifeste UAC `requireAdministrator`. Les exécutables de développement et de test ne demandent pas d'élévation UAC. Les compilations release n'ouvrent pas de console.

### Linux

Sur Debian/Ubuntu :

```sh
sudo apt install build-essential pkg-config libgtk-3-dev libayatana-appindicator3-dev libglib2.0-bin pkexec libcap2-bin
cargo build --release
./target/release/wifinx-validator
```

Sur Arch Linux, installez `base-devel`, `pkgconf`, `gtk3`, `libayatana-appindicator`, `glib2`, `polkit` et `libcap`. Les noms des paquets varient selon la distribution. Les binaires Linux sont liés dynamiquement ; compilez localement si les exigences glibc ou bibliothèques système d'une archive ne correspondent pas à votre système.

Pour accorder manuellement la capacité d'utiliser les ports privilégiés :

```sh
sudo setcap cap_net_bind_service=+ep target/release/wifinx-validator
./target/release/wifinx-validator
```

### macOS

Installez les outils de ligne de commande Xcode et Rust stable, puis compilez sur le Mac cible :

```sh
xcode-select --install
cargo build --release
./target/release/wifinx-validator
```

Les archives de release fournissent le bundle `.app` signé ad hoc ; une compilation Cargo locale produit l’exécutable. macOS 11 ou ultérieur est requis.

### Contrôles

```sh
cargo fmt --check
cargo test --no-default-features
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Pour vérifier le cœur sans accès à crates.io :

```sh
./scripts/test-core.sh          # tous les tests du cœur, y compris sockets loopback
./scripts/test-core.sh --pure   # tests sans ouverture de sockets
```

`scripts/check-windows-build.ps1` vérifie les imports et le manifeste administrateur d'une compilation release Windows avec les outils Visual Studio et le SDK Windows installés. Le workflow `.github/workflows/ci.yml` compile les cinq cibles natives et publie leurs archives ainsi que `SHA256SUMS` dans une GitHub Release uniquement lors du push d’un tag strict `vX.X.X` correspondant à la version de `Cargo.toml`. Aucun build ni publication de release ne démarre sur une branche ou une pull request. Compilation et contrôles automatiques ne remplacent pas les essais de l'interface, de la notification, des pilotes graphiques et de la console sur le matériel cible.

Pour publier la version `0.1.0` après vérification du code et de la version Cargo, le responsable peut exécuter (remplacez `TeslaFr` par le nom de votre remote si nécessaire) :

```sh
git tag v0.1.0
git push TeslaFr v0.1.0
```

## Licence et attribution

MIT ; voir [LICENSE](LICENSE) et [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Le comportement DNS/HTTP est adapté de [setup_pi_wan.sh](https://github.com/TheNxtBigThing8/Offline-WAN-Emulator/blob/main/setup_pi_wan.sh), copyright 2026 TheNxtBigThing8, sous licence MIT.

Bibliothèques graphiques : [eframe](https://github.com/emilk/egui) et [tray-icon](https://github.com/tauri-apps/tray-icon). Références permissions/compilation : [runtime C statique Rust](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes), [manifestes UAC Microsoft](https://learn.microsoft.com/en-us/windows/win32/secbp/running-with-administrator-privileges/) et [pkexec/Polkit](https://www.freedesktop.org/software/polkit/docs/master/pkexec.1.html).
