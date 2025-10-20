# Installation Guide

## Overview

Edge Policy Hub can be deployed in three ways:

1. **Native Installation** (Linux/Windows/macOS) — recommended for production
2. **Docker Compose** — recommended for development and containerized environments
3. **Manual Installation** — for custom deployments

## System Requirements

### Minimum

- CPU: 2 cores
- RAM: 4 GB
- Disk: 10 GB available
- OS: Linux (Ubuntu 20.04+, Debian 11+, RHEL 8+), Windows 10+, macOS 10.15+
- Network: Stable broadband connection

### Recommended

- CPU: 4 cores
- RAM: 8–16 GB
- Disk: 50 GB SSD
- Network: 100 Mbps+

### Ports Required

- 8080: HTTP proxy (external)
- 1883: MQTT TCP (external)
- 8883: MQTT TLS (external, optional)
- 8181: Enforcer API (internal)
- 8182: Audit Store API (internal)
- 8183: Quota Tracker API (internal)

---

## Native Installation

### Linux (Debian/Ubuntu)

```bash
# Download latest release
wget https://github.com/acailic/edge-policy-hub/releases/latest/download/edge-policy-hub_1.0.0_amd64.deb

# Install package
sudo dpkg -i edge-policy-hub_1.0.0_amd64.deb

# Install dependencies if needed
sudo apt-get install -f

# Start services
sudo systemctl start edge-policy-hub.target

# Enable auto-start on boot
sudo systemctl enable edge-policy-hub.target

# Check status
sudo systemctl status edge-policy-hub.target
```

**Post-Installation**

- Binaries: `/usr/local/bin/edge-policy-*`
- Data directory: `/var/lib/edge-policy-hub/`
- Configuration: `/etc/edge-policy-hub/`
- Logs: `journalctl -u edge-policy-hub.target -f`
- Desktop UI: Launch from applications menu or run `edge-policy-ui`

### Linux (RHEL/CentOS/Fedora)

```bash
# Download latest release
wget https://github.com/acailic/edge-policy-hub/releases/latest/download/edge-policy-hub-1.0.0-1.x86_64.rpm

# Install package
sudo rpm -i edge-policy-hub-1.0.0-1.x86_64.rpm

# Or using dnf
sudo dnf install edge-policy-hub-1.0.0-1.x86_64.rpm

# Start services
sudo systemctl start edge-policy-hub.target
sudo systemctl enable edge-policy-hub.target
```

### Windows

1. Download `Edge_Policy_Hub_1.0.0_x64_en-US.msi` from the latest release.
2. Double-click the installer and follow the wizard:
   - Accept the license agreement.
   - Choose the installation directory (default: `C:\Program Files\Edge Policy Hub`).
   - Select components (all services + desktop UI).
   - Click **Install** (requires Administrator privileges).
3. The installer automatically:
   - Installs backend service binaries.
   - Registers the Windows Service (`EdgePolicyHub`).
   - Configures auto-start on boot.
   - Creates a desktop shortcut for the UI.
   - Adds firewall rules for proxy (8080) and MQTT (1883) ports.
4. Launch the desktop UI from the Start Menu or desktop shortcut.

**Post-Installation**

- Service: Windows Service (`EdgePolicyHub`)
- Data directory: `C:\ProgramData\Edge Policy Hub\`
- Configuration: `C:\Program Files\Edge Policy Hub\config\`
- Logs: `C:\Program Files\Edge Policy Hub\logs\`
- Manage service: `services.msc` and search for “Edge Policy Hub”

**Verify Installation**

```powershell
# Check service status
Get-Service -Name EdgePolicyHub

# View logs
Get-Content "C:\Program Files\Edge Policy Hub\logs\launcher.log" -Tail 50

# Test enforcer
Invoke-WebRequest -Uri http://localhost:8181/health
```

### macOS

1. Download `Edge_Policy_Hub_1.0.0_x64.dmg`.
2. Double-click to mount the DMG.
3. Drag “Edge Policy Hub” into the Applications folder.
4. Launch from Applications.
5. On first launch, macOS Gatekeeper may prompt:
   - Open **System Preferences → Security & Privacy → Allow**.
   - Or right-click app → **Open** → confirm.
6. The app:
   - Starts backend services automatically.
   - Creates data directory at `~/Library/Application Support/com.edgepolicyhub.app/`.
   - Presents the desktop UI.

**Post-Installation**

- Services run as background processes managed by the Tauri app.
- Data directory: `~/Library/Application Support/com.edgepolicyhub.app/`
- Logs: `~/Library/Logs/Edge Policy Hub/`

**Verify Installation**

```bash
# Check if services are running
ps aux | grep edge-policy

# Test enforcer
curl http://localhost:8181/health
```

---

## Docker Compose Installation

Refer to [infra/docker/README.md](../infra/docker/README.md) for full instructions.

```bash
cd infra/docker
cp .env.example .env
# Edit .env with your settings
docker-compose up -d
```

---

## Manual Installation

### Build from Source

**Prerequisites**

- Rust 1.75+
- Node.js 18+
- pnpm (or npm)

```bash
# Clone repository
git clone https://github.com/acailic/edge-policy-hub.git
cd edge-policy-hub

# Build backend services
cargo build --release --workspace

# Build Tauri UI
cd apps/tauri-ui
pnpm install
pnpm tauri build

# Binaries are in:
# - target/release/edge-policy-*
# - apps/tauri-ui/src-tauri/target/release/bundle/
```

### Manual Service Setup (Linux)

```bash
# Copy binaries
sudo cp target/release/edge-policy-* /usr/local/bin/

# Create user and directories
sudo useradd --system --no-create-home edge-policy
sudo mkdir -p /var/lib/edge-policy-hub/{config/tenants.d,data/{audit,quota}}
sudo chown -R edge-policy:edge-policy /var/lib/edge-policy-hub

# Copy systemd service files
sudo cp infra/systemd/*.service /etc/systemd/system/
sudo cp infra/systemd/*.target /etc/systemd/system/

# Generate HMAC secret
openssl rand -base64 32 | sudo tee /etc/edge-policy-hub/hmac-secret
sudo chown edge-policy:edge-policy /etc/edge-policy-hub/hmac-secret
sudo chmod 600 /etc/edge-policy-hub/hmac-secret

# Enable and start services
sudo systemctl daemon-reload
sudo systemctl enable edge-policy-hub.target
sudo systemctl start edge-policy-hub.target
```

---

## Configuration

### Environment Variables

All services are configured via environment variables. See individual service READMEs for full reference:

- [Enforcer Configuration](../services/enforcer/README.md)
- [Proxy HTTP Configuration](../services/proxy-http/README.md)
- [MQTT Bridge Configuration](../services/bridge-mqtt/README.md)
- [Audit Store Configuration](../services/audit-store/README.md)
- [Quota Tracker Configuration](../services/quota-tracker/README.md)

### TLS/mTLS Setup

For production deployments with TLS:

```bash
# Create CA
openssl req -x509 -newkey rsa:4096 -days 365 -nodes -keyout ca-key.pem -out ca-cert.pem -subj "/CN=Edge Policy CA"

# Create server certificate
openssl req -newkey rsa:4096 -nodes -keyout server-key.pem -out server-req.pem -subj "/CN=edge-policy-gateway"
openssl x509 -req -in server-req.pem -days 365 -CA ca-cert.pem -CAkey ca-key.pem -CAcreateserial -out server-cert.pem

# Create client certificate (for mTLS)
openssl req -newkey rsa:4096 -nodes -keyout client-key.pem -out client-req.pem -subj "/CN=tenant-a"
openssl x509 -req -in client-req.pem -days 365 -CA ca-cert.pem -CAkey ca-key.pem -CAcreateserial -out client-cert.pem -extfile <(echo "subjectAltName=URI:tenant:tenant-a")
```

Configure services with the generated certificate paths and restart to apply changes.

---

## Verification

### Check Service Health

**Linux (systemd)**

```bash
sudo systemctl status edge-policy-hub.target
journalctl -u edge-policy-enforcer -f
```

**Windows**

```powershell
Get-Service -Name EdgePolicyHub
Get-Content "C:\Program Files\Edge Policy Hub\logs\launcher.log" -Tail 50
```

**Docker**

```bash
docker-compose ps
docker-compose logs -f
```

### Test Endpoints

```bash
# Enforcer health
curl http://localhost:8181/health

# Audit store health
curl http://localhost:8182/health

# Quota tracker health
curl http://localhost:8183/health

# HTTP proxy (requires tenant and policy)
curl -H "X-Tenant-ID: tenant-a" http://localhost:8080/api/test

# MQTT bridge (requires MQTT client)
mosquitto_pub -h localhost -p 1883 -i "tenant-a/device-1" -t "tenant-a/test" -m "hello"
```

---

## Upgrading

### Native Installation

**Linux**

```bash
# Download new version
wget https://github.com/acailic/edge-policy-hub/releases/latest/download/edge-policy-hub_1.1.0_amd64.deb

# Upgrade package (preserves data)
sudo dpkg -i edge-policy-hub_1.1.0_amd64.deb

# Restart services
sudo systemctl restart edge-policy-hub.target
```

**Windows**

- Download the new `.msi` installer.
- Run the installer — select **Upgrade** when prompted.
- Data is preserved automatically.

**macOS**

- Download the new `.dmg`.
- Replace the app in Applications.
- Data remains in Application Support.

### Docker Compose

```bash
# Pull latest images
docker-compose pull

# Recreate containers (preserves volumes)
docker-compose up -d
```

### Self-Update (Tauri UI)

The desktop UI checks for updates automatically:

1. On startup, checks GitHub releases for a newer version.
2. If available, shows an update notification.
3. After user confirmation, downloads and installs the update.
4. Restarts the application with the new version.

Manual update check: **Settings → Check for Updates**.

---

## Uninstallation

### Linux

```bash
# Debian/Ubuntu
sudo apt-get remove edge-policy-hub

# Remove data as well
sudo apt-get purge edge-policy-hub
```

Or use the uninstall script:

```bash
sudo /usr/local/bin/edge-policy-hub-uninstall.sh
```

### Windows

**Control Panel**

1. Control Panel → Programs → Uninstall a program.
2. Select “Edge Policy Hub” and click **Uninstall**.
3. Follow prompts for data retention.

**PowerShell**

```powershell
& "C:\Program Files\Edge Policy Hub\uninstall.ps1"
```

### macOS

```bash
# Remove app
rm -rf /Applications/Edge\ Policy\ Hub.app

# Remove data (optional)
rm -rf ~/Library/Application\ Support/com.edgepolicyhub.app
rm -rf ~/Library/Logs/Edge\ Policy\ Hub
```

### Docker Compose

```bash
# Stop and remove containers (keep data)
docker-compose down

# Remove everything including volumes
docker-compose down -v
```

---

## Troubleshooting

### Installation Fails

**Linux**

- Check dependencies: `sudo apt-get install -f`
- Verify disk space: `df -h`
- Check permissions: `ls -la /var/lib/edge-policy-hub`
- Review install log: `/var/log/edge-policy-hub/install.log`

**Windows**

- Run installer as Administrator.
- Check Windows Event Viewer for errors.
- Verify the WebView2 runtime is installed.
- Review install log: `C:\Program Files\Edge Policy Hub\logs\install.log`

### Services Not Starting

**Linux**

```bash
sudo systemctl status edge-policy-enforcer
journalctl -u edge-policy-enforcer -n 100
sudo netstat -tuln | grep -E '8080|1883|8181'
sudo -u edge-policy ls /var/lib/edge-policy-hub
```

**Windows**

```powershell
Get-Service -Name EdgePolicyHub
Get-EventLog -LogName Application -Source EdgePolicyHub -Newest 50
netstat -ano | findstr -E "8080|1883|8181"
```

### Permission Errors

- Ensure the `edge-policy` user owns data directories.
- Check SELinux/AppArmor policies on hardened systems.
- Verify systemd unit `User` and `Group` settings.

### Port Conflicts

**Linux (systemd)**

```bash
sudo systemctl edit edge-policy-proxy-http

# Add override
[Service]
Environment="PROXY_PORT=9080"

sudo systemctl daemon-reload
sudo systemctl restart edge-policy-proxy-http
```

**Docker**

```yaml
services:
  proxy-http:
    ports:
      - "9080:8080"  # Map host port 9080 to container port 8080
```

---

## Next Steps

After installation:

1. Launch the desktop UI and configure tenants.
2. Set quota limits and thresholds.
3. Define ABAC policies using the policy builder.
4. Deploy policies and validate enforcement.
5. Monitor real-time decisions and quota usage.

For a detailed walkthrough, see the [Getting Started Guide](getting-started.md).

---

## Support

- **Documentation**: [docs/](../docs/)
- **Issues**: [GitHub Issues](https://github.com/acailic/edge-policy-hub/issues)
- **Discussions**: [GitHub Discussions](https://github.com/acailic/edge-policy-hub/discussions)
