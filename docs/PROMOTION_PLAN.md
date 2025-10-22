# Edge Policy Hub: Promotion and Validation Plan

This document outlines a strategic plan to promote the Edge Policy Hub and validate its effectiveness through real-world scenario testing.

## Part 1: Promotion and Publication Strategy

The goal is to build awareness and drive adoption among the target audience: operators and developers managing edge deployments, IoT fleets, and remote offices.

### Phase 1: Pre-Launch Preparation (1-2 Weeks)

Before announcing the project widely, we need to ensure all public-facing materials are polished and ready for an influx of visitors.

**1. Solidify the Narrative:**
   - **Key Message:** "A simple, secure, click-to-deploy edge gateway for policy-as-code. Enforce data residency, control costs, and secure your edge infrastructure, even when offline."
   - **Elevator Pitch:** "Edge Policy Hub is an open-source, multi-tenant access control gateway for your edge sites. It uses OPA/Rego to enforce policies for HTTP and MQTT traffic, runs offline, and is managed through a simple desktop UI."

**2. Enhance Project Assets:**
   - **README Review:** The current README is excellent. Ensure all links are valid and the installation instructions have been tested on clean machines for all platforms (Windows, macOS, Debian, RHEL).
   - **Website/Landing Page:** Create a simple, clean GitHub Pages site from the `README.md` to serve as a professional-looking landing page.
   - **Screencast/Demo Video (2-3 minutes):**
     - *Scene 1:* Show the problem: a remote site with devices sending data, highlighting the compliance/security risks.
     - *Scene 2:* Show the solution: Install Edge Policy Hub with one click using the Tauri installer.
     - *Scene 3:* Use the UI to create a tenant and a data residency policy (e.g., "Block non-EU traffic").
     - *Scene 4:* Show traffic being blocked in the UI's audit log.
     - *Scene 5:* Add a cost-control policy (e.g., "Max 1GB upload") and show it working.
     - *Call to Action:* End with a link to the GitHub repository.
   - **Blog Post Draft:** Write a launch blog post titled something like: "Introducing Edge Policy Hub: Offline-First, Policy-as-Code for the Edge".
     - Explain the "why": The gap in edge security and compliance.
     - Explain the "what": A tour of the key features.
     - Explain the "how": A quick-start guide.
     - Host it on a platform like Medium, dev.to, or a personal blog.

### Phase 2: Launch Day

Coordinate announcements across multiple platforms to maximize reach.

**1. Target Communities:**
   - **Hacker News:** Post a link to the GitHub repository with the title: "Show HN: An open-source, offline-first edge gateway with OPA". Be prepared to answer questions and engage in the comments.
   - **Reddit:**
     - `/r/rust`: "I built a multi-tenant edge gateway in Rust with Tauri and OPA/Rego."
     - `/r/selfhosted`: "Edge Policy Hub: A self-hostable policy gateway for your remote/home lab."
     - `/r/iot`: "An open-source solution for enforcing access policies on IoT devices at the edge."
     - `/r/devops`: "Policy-as-code for edge deployments using OPA, but with an easy-to-use UI."
   - **Twitter/X:** Announce the launch, tag relevant accounts (@rustlang, @openpolicyagent, @TauriApps), and post the demo video.
   - **Specialized Forums:** Post in OPA/Rego community channels (e.g., Slack).

**2. Publish Content:**
   - Publish the blog post on the chosen platform.
   - Upload the demo video to YouTube.
   - Link to the blog post and video in all social media announcements.

### Phase 3: Post-Launch Engagement

- **Monitor Channels:** Actively respond to comments, questions, and issues on GitHub, Reddit, and Hacker News.
- **Encourage Contributions:** Create "good first issue" tickets for simple bugs or documentation improvements to lower the barrier for new contributors.
- **Write Follow-up Content:** Based on community feedback, write more in-depth articles on specific topics (e.g., "Deep Dive into the Policy DSL," "Setting up a Resilient Edge Gateway for an IoT Fleet").

---

## Part 2: Real-World Scenario Validation Plan

The goal is to build confidence in the project's stability, performance, and correctness by simulating production-like workloads and conditions.

### Scenario 1: Multi-Tenant Retail Chain

- **Context:** A retail company deploys Edge Policy Hub in each of its 50 stores. Each store is a tenant. The gateway handles guest Wi-Fi traffic (HTTP) and inventory scanner data (MQTT).
- **Validation Objectives:**
  1. **Tenant Isolation:** Ensure traffic and policies for `store-01` are completely isolated from `store-02`.
  2. **Cost Control:** Guest Wi-Fi is throttled. Each store has a 500 GB/month egress quota for large file uploads to corporate FTP servers.
  3. **Data Residency:** Scanners in EU stores can only send data to EU-based endpoints.
- **Test Setup:**
  - Use Docker Compose to spin up the full Edge Policy Hub stack.
  - Create 50 tenants (`store-01` to `store-50`) via a script.
  - **HTTP Simulation:** Use a load generator like `k6` or `vegeta` to simulate guest Wi-Fi users. Script scenarios where users try to access blocked sites or upload large files that exceed the quota.
  - **MQTT Simulation:** Use an MQTT client library (e.g., `mqttx` CLI or a Python script with `paho-mqtt`) to simulate 10-20 inventory scanners per store, each sending periodic updates.
- **Success Criteria:**
  - ✅ Policy violations are correctly logged in the audit store for the specific tenant.
  - ✅ Quotas are enforced accurately; traffic is blocked once the limit is reached.
  - ✅ The enforcer's p99 latency remains < 10ms under load.
  - ✅ The host system's CPU and memory usage remain stable.

### Scenario 2: Smart Factory (IIoT)

- **Context:** A manufacturing plant uses the gateway to manage traffic from thousands of sensors and PLCs on the factory floor, which have intermittent network connectivity.
- **Validation Objectives:**
  1. **Offline-First Resilience:** The gateway must continue to enforce policies when its connection to the central cloud is severed.
  2. **High-Throughput MQTT:** The MQTT bridge must handle a high volume of small, frequent messages without significant latency.
  3. **Zero-Trust Policy:** Only authenticated devices (`subject.device_id`) can publish to specific topics (`resource.id`).
- **Test Setup:**
  - Deploy the services on a single-board computer (e.g., Raspberry Pi 4) to simulate a real edge hardware environment.
  - **Network Failure Simulation:** Use `iptables` or `tc` on Linux to introduce network latency or completely drop packets to simulate a WAN outage.
  - **MQTT Load:** Use a high-performance MQTT benchmark tool to simulate 1,000+ devices connecting and publishing messages at a rate of 10 messages/sec.
  - **Policy:** Define a strict policy where `device-group-A` can only publish to `/factory/line1/temp` and `device-group-B` can only publish to `/factory/line2/pressure`.
- **Success Criteria:**
  - ✅ During the simulated WAN outage, the gateway continues to enforce policies for local traffic.
  - ✅ When connectivity is restored, the audit log is successfully uploaded to a mock cloud endpoint.
  - ✅ The MQTT bridge handles the load without message loss, and policy violations (e.g., device from group A trying to publish to line 2) are blocked and logged.

### Scenario 3: Security Audit and Tamper-Proofing

- **Context:** A security team needs to verify that the audit logs are secure and that the system is resilient to basic attacks.
- **Validation Objectives:**
  1. **Log Integrity:** Ensure that the signed audit logs cannot be modified without detection.
  2. **Policy Integrity:** Ensure that policy bundles cannot be tampered with.
- **Test Setup:**
  - Run the `audit-store` service and generate some traffic.
  - **Manual Tampering:**
    - Stop the service.
    - Manately edit the raw audit log file on disk to change an event detail.
    - Restart the service.
  - **Policy Tampering:**
    - Manually edit a compiled `bundle.tar.gz` in the bundles directory.
- **Success Criteria:**
  - ✅ The audit service detects the log tampering on restart and raises an error.
  - ✅ The enforcer service fails to load the tampered policy bundle, or loads the last known-good version, and logs a signature validation error.
