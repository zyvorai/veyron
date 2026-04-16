# Network Management & Monitoring

VMRogue provides comprehensive network management capabilities for KubeVirt VMs, including interface management, bandwidth monitoring, traffic analysis, and advanced Cilium-based network policies.

## Features

### 1. Network Interface Management (`network-list`, `network-get`)

List and inspect network interfaces attached to VMs.

**Usage:**
```bash
# List all interfaces for a VM
vmrogue network-list my-vm

# Show details for a specific interface
vmrogue network-get my-vm eth0

# JSON/YAML output
vmrogue network-list my-vm --output json
```

**Supported Interface Types:**
- **Bridge** - Standard bridge networking
- **Masquerade** - NAT-based networking
- **SR-IOV** - Direct hardware passthrough for high performance
- **Multus** - Multiple network attachments

**Example Output:**
```
╭─────────────────────────────────────╮
│  Network Interfaces: my-vm          │
╰─────────────────────────────────────╯

NAME         NETWORK          MAC ADDRESS           IP ADDRESS          TYPE         STATE
-------------------------------------------------------------------------------------------------------
eth0         pod-network      52:54:00:12:34:56     10.244.0.5         bridge       UP
eth1         storage-network  52:54:00:12:34:57     192.168.1.10       multus       UP
```

### 2. Bandwidth Monitoring (`network-bandwidth`)

Monitor network bandwidth usage in real-time.

**Usage:**
```bash
# Show bandwidth for all interfaces
vmrogue network-bandwidth my-vm

# Monitor specific interface
vmrogue network-bandwidth my-vm --interface eth0

# Watch mode (continuous updates)
vmrogue network-bandwidth my-vm --watch --interval 5
```

**Metrics Tracked:**
- **RX/TX Bytes** - Total received/transmitted data
- **RX/TX Packets** - Total packet counts
- **RX/TX Errors** - Error counts
- **RX/TX Dropped** - Dropped packet counts
- **Error Rate** - Percentage of packets with errors
- **Drop Rate** - Percentage of dropped packets
- **Bandwidth Rates** - Current transfer rates (MB/s, GB/s)

**Example Output:**
```
╭─────────────────────────────────────╮
│  Network Bandwidth: my-vm           │
╰─────────────────────────────────────╯

INTERFACE       RX              TX              RX RATE      TX RATE
---------------------------------------------------------------------------
eth0            1.40 GB         762.94 MB       125 MB/s     80 MB/s

Statistics:
  RX Packets:  1200000
  TX Packets:  600000
  RX Errors:   5
  TX Errors:   2
  Error Rate:  0.000%
```

**Features:**
- Real-time bandwidth monitoring
- Peak and average rate calculation
- Link utilization percentage
- Error and drop detection
- Historical statistics

### 3. Traffic Analysis (`network-traffic`)

Analyze network traffic flows and identify top talkers.

**Usage:**
```bash
# Analyze traffic for the last 15 minutes
vmrogue network-traffic my-vm

# Analyze specific interface
vmrogue network-traffic my-vm --interface eth0

# Custom time period and top N talkers
vmrogue network-traffic my-vm --period 1h --top 20

# JSON/YAML output
vmrogue network-traffic my-vm --output json
```

**Analysis Capabilities:**
- **Flow Tracking** - Track individual network flows
- **Protocol Breakdown** - Traffic by protocol (TCP/UDP/ICMP)
- **Top Talkers** - Identify hosts with most traffic
- **Traffic Patterns** - Analyze communication patterns
- **Bandwidth Usage** - Per-flow bandwidth consumption

**Example Output:**
```
╭──────────────────────────────────────╮
│  Network Traffic Analysis: my-vm     │
│  Period: 15m                         │
╰──────────────────────────────────────╯

Traffic Summary:
  Total Flows:    3
  Active Flows:   3
  Total Bytes:    163.15 MB
  Total Packets:  121200

Protocol Breakdown:
  TCP      163.05 MB (99.9%)
  UDP      1.43 MB (0.1%)

Top 10 Talkers:
IP ADDRESS          SENT            RECEIVED        TOTAL
----------------------------------------------------------------------
10.244.0.5          163.15 MB       0 B             163.15 MB
10.244.0.8          0 B             114.44 MB       114.44 MB
8.8.8.8             0 B             47.68 MB        47.68 MB
```

### 4. Network Policies (`network-policies`, `network-policy`)

Manage Kubernetes and Cilium network policies.

**Usage:**
```bash
# List all network policies
vmrogue network-policies

# Show all namespaces
vmrogue network-policies --all-namespaces

# Show policy details
vmrogue network-policy web-policy

# YAML/JSON output
vmrogue network-policy web-policy --output yaml
```

**Policy Features:**
- **Ingress Rules** - Control incoming traffic
- **Egress Rules** - Control outgoing traffic
- **CIDR-based Rules** - IP range filtering
- **Port-based Rules** - Port and protocol filtering
- **Label Selectors** - Target VMs by labels

**Example Output:**
```
╭─────────────────────────────────────╮
│  Network Policies                   │
╰─────────────────────────────────────╯

NAME                      INGRESS      EGRESS       SELECTOR
----------------------------------------------------------------------
web-policy                1            0            app=web
database-policy           2            1            app=database
```

## Cilium Integration

VMRogue provides advanced network policy capabilities through Cilium integration, leveraging eBPF for high-performance, identity-based security.

### Cilium Features

**1. Identity-Based Security**
- Security based on pod identity, not IP addresses
- Automatic identity assignment
- No dependency on IP address management

**2. L7 Protocol Awareness**
- **HTTP/HTTPS** - Method, path, host filtering
- **Kafka** - Topic, API key filtering
- **DNS** - FQDN-based policies
- **gRPC** - Method filtering

**3. DNS-Based Policies**
- Allow traffic to specific FQDNs
- Wildcard domain matching
- Automatic DNS resolution

**4. Service-Based Policies**
- Reference Kubernetes services directly
- No need to specify IPs or ports
- Automatic service discovery

### Cilium Policy Examples

**Example 1: HTTP L7 Policy**

Allow only GET requests to `/api/*`:

```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: web-api-policy
spec:
  endpointSelector:
    matchLabels:
      app: web
  ingress:
  - fromEndpoints:
    - matchLabels:
        role: frontend
    toPorts:
    - ports:
      - port: "443"
        protocol: TCP
      rules:
        http:
        - method: "GET"
          path: "/api/.*"
```

**Example 2: DNS-Based Egress**

Allow traffic only to specific external services:

```yaml
apiVersion: cilium.io/v2
kind:CiliumNetworkPolicy
metadata:
  name: external-access-policy
spec:
  endpointSelector:
    matchLabels:
      app: backend
  egress:
  - toFQDNs:
    - matchPattern: "*.googleapis.com"
    - matchName: "api.stripe.com"
  - toPorts:
    - ports:
      - port: "443"
        protocol: TCP
```

**Example 3: Kafka L7 Policy**

Control access to specific Kafka topics:

```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: kafka-policy
spec:
  endpointSelector:
    matchLabels:
      app: payment-service
  egress:
  - toEndpoints:
    - matchLabels:
        app: kafka
    toPorts:
    - ports:
      - port: "9092"
        protocol: TCP
      rules:
        kafka:
        - topic: "payments"
        - topic: "orders"
```

**Example 4: Service-Based Policy**

Reference Kubernetes services:

```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: service-access-policy
spec:
  endpointSelector:
    matchLabels:
      app: web
  egress:
  - toServices:
    - namespace: default
      name: database
    - namespace: auth
      name: auth-service
```

### Cilium Policy Types

| Type | Description | Use Case |
|------|-------------|----------|
| **CiliumNetworkPolicy** | Namespace-scoped | Application-specific policies |
| **CiliumClusterwideNetworkPolicy** | Cluster-wide | Global security rules |
| **L3/L4 Policies** | IP/Port filtering | Basic network segmentation |
| **L7 Policies** | Application protocol | API security, method filtering |
| **DNS Policies** | FQDN-based | External service access |

### Creating Cilium Policies with VMRogue

```rust
use vmrogue::network::cilium::*;

// Create HTTP L7 policy
let http_rule = HTTPRule::new()
    .method("GET")
    .path("/api/*")
    .host("api.example.com");

let port_rule = PortRule::new()
    .add_port(Port::tcp(443))
    .with_http(vec![http_rule]);

let ingress = CiliumIngressRule::new()
    .from_endpoint(
        EndpointSelector::new()
            .with_label("role", "frontend")
    )
    .to_port(port_rule);

let policy = CiliumNetworkPolicy::new("api-policy")
    .with_selector(
        VMSelector::default()
            .with_label("app", "api")
    )
    .add_ingress(ingress);

// Add to manager
let mut manager = CiliumPolicyManager::new();
manager.add_policy(policy);

// Generate YAML
let yaml = manager.generate_yaml("api-policy");
```

## Best Practices

### 1. Network Interface Management

**Multi-Network Configuration:**
```bash
# Production setup: pod network + storage network
# eth0: pod-network (default)
# eth1: storage-network (high-bandwidth)
# eth2: management-network (isolated)
```

**Interface Naming:**
- Use descriptive names: `storage`, `management`, `external`
- Follow consistent naming conventions
- Document network purposes

### 2. Bandwidth Monitoring

**Regular Monitoring:**
```bash
# Set up monitoring cron job
*/5 * * * * vmrogue network-bandwidth production-vm >> /var/log/bandwidth.log

# Alert on high bandwidth
vmrogue network-bandwidth my-vm | \
  awk '/RX RATE.*GB\/s/ {system("alert-high-bandwidth.sh")}'
```

**Capacity Planning:**
- Monitor peak usage times
- Track bandwidth trends
- Plan for growth

### 3. Traffic Analysis

**Security Monitoring:**
```bash
# Daily traffic analysis
vmrogue network-traffic my-vm --period 24h --top 50 --output json > traffic-report.json

# Identify unusual patterns
jq '.top_talkers[] | select(.total_bytes > 10000000000)' traffic-report.json
```

**Performance Optimization:**
- Identify chatty services
- Optimize inter-service communication
- Reduce unnecessary traffic

### 4. Network Policies

**Defense in Depth:**
```yaml
# Layer 1: Namespace isolation
# Layer 2: Pod-to-pod policies
# Layer 3: L7 application policies
```

**Policy Testing:**
```bash
# Test in dry-run mode
kubectl apply --dry-run=client -f policy.yaml

# Validate with network policy viewer
kubectl get networkpolicies
```

**Gradual Rollout:**
1. Start with monitoring/logging mode
2. Test with non-production workloads
3. Gradually enable enforcement
4. Monitor for blocked connections

### 5. Cilium-Specific Best Practices

**Start with L3/L4, Add L7 Gradually:**
- Begin with IP/port-based policies
- Add L7 rules for critical services
- Monitor performance impact

**Use DNS Policies for External Access:**
```yaml
# Better than maintaining IP lists
toFQDNs:
- matchPattern: "*.aws.amazon.com"
- matchPattern: "*.storage.googleapis.com"
```

**Leverage Identity-Based Security:**
```yaml
# Use labels, not IPs
fromEndpoints:
- matchLabels:
    role: frontend
    tier: web
```

**Monitor Cilium Metrics:**
```bash
# Check policy effectiveness
cilium monitor --type policy-verdict

# View flow logs
hubble observe --namespace default
```

## Troubleshooting

### Network Interface Issues

**Interface Not Showing Up:**
```bash
# Check VM status
vmrogue status my-vm

# Verify network attachment
kubectl get network-attachment-definitions

# Check for errors
kubectl describe vmi my-vm
```

**IP Address Not Assigned:**
```bash
# Check DHCP/IPAM
kubectl logs -n kube-system -l app=kube-ipam

# Verify network configuration
vmrogue network-get my-vm eth0
```

### Bandwidth Issues

**High Error Rate:**
```bash
# Check physical network
ethtool eth0

# Verify MTU settings
ip link show eth0

# Check for packet drops
netstat -i
```

**Unexpectedly Low Bandwidth:**
```bash
# Check for QoS limits
tc qdisc show dev eth0

# Verify network policy restrictions
vmrogue network-policies
```

### Traffic Analysis Issues

**Missing Flows:**
```bash
# Verify flow collection
kubectl logs -n kube-system -l app=flow-collector

# Check iptables rules
iptables -L -n -v
```

**Inaccurate Statistics:**
```bash
# Sync time across nodes
chronyc tracking

# Verify metrics collection
kubectl top pods
```

### Network Policy Issues

**Traffic Blocked Unexpectedly:**
```bash
# Check applied policies
vmrogue network-policies --all-namespaces

# View policy details
vmrogue network-policy my-policy --output yaml

# Test connectivity
kubectl run test --rm -it --image=busybox -- wget -O- my-service
```

**Policy Not Applied:**
```bash
# Verify label selectors
kubectl get pods --show-labels

# Check policy syntax
kubectl describe networkpolicy my-policy

# Restart network plugin
kubectl rollout restart -n kube-system daemonset/cilium
```

### Cilium-Specific Issues

**L7 Policy Not Working:**
```bash
# Check Cilium status
cilium status

# Verify L7 proxy
cilium endpoint list

# View policy enforcement
cilium policy get
```

**DNS Policy Not Resolving:**
```bash
# Check DNS proxy
cilium service list

# Verify FQDN cache
cilium fqdn cache list

# Test DNS resolution
cilium monitor --type l7 --to-fqdn
```

## Advanced Topics

### Multi-Network Setup

```yaml
# Multus network attachment definition
apiVersion: k8s.cni.cncf.io/v1
kind: NetworkAttachmentDefinition
metadata:
  name: storage-network
spec:
  config: |
    {
      "cniVersion": "0.3.1",
      "type": "macvlan",
      "master": "eth1",
      "mode": "bridge",
      "ipam": {
        "type": "host-local",
        "subnet": "192.168.1.0/24"
      }
    }
```

### SR-IOV Configuration

```yaml
# SR-IOV Network Node Policy
apiVersion: sriovnetwork.openshift.io/v1
kind: SriovNetworkNodePolicy
metadata:
  name: policy-1
  namespace: openshift-sriov-network-operator
spec:
  deviceType: netdevice
  nicSelector:
    deviceID: "1583"
    vendor: "8086"
  nodeSelector:
    feature.node.kubernetes.io/network-sriov.capable: "true"
  numVfs: 8
  priority: 99
  resourceName: sriov_net
```

### Service Mesh Integration

Cilium can be integrated with service meshes for enhanced observability:

```yaml
# Enable Hubble for observability
hubble:
  enabled: true
  metrics:
    enabled:
    - dns
    - drop
    - tcp
    - flow
    - http
```

## API Reference

### NetworkInterface

```rust
pub struct NetworkInterface {
    pub name: String,
    pub network: String,
    pub mac_address: String,
    pub ip_address: Option<String>,
    pub interface_type: InterfaceType,
    pub model: String,
    pub mtu: u32,
    pub state: InterfaceState,
}

impl NetworkInterface {
    pub fn new(name: impl Into<String>) -> Self
    pub fn with_network(self, network: impl Into<String>) -> Self
    pub fn is_connected(&self) -> bool
}
```

### BandwidthMetrics

```rust
pub struct BandwidthMetrics {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

impl BandwidthMetrics {
    pub fn format_bytes(bytes: u64) -> String
    pub fn format_rate(bytes_per_sec: u64) -> String
    pub fn error_rate(&self) -> f64
}
```

### CiliumNetworkPolicy

```rust
pub struct CiliumNetworkPolicy {
    pub name: String,
    pub endpoint_selector: VMSelector,
    pub ingress: Vec<CiliumIngressRule>,
    pub egress: Vec<CiliumEgressRule>,
}

impl CiliumNetworkPolicy {
    pub fn new(name: impl Into<String>) -> Self
    pub fn add_ingress(self, rule: CiliumIngressRule) -> Self
    pub fn add_egress(self, rule: CiliumEgressRule) -> Self
}
```

## REST API

The network API is wired to real VMI interface data from KubeVirt.

### List Network Interfaces

```
GET /network/interfaces
```

Returns network interfaces extracted from VirtualMachineInstance status, including:
- IP address, MAC address, interface name
- Associated VM name and namespace
- Interface type and network name

### Bandwidth Monitoring

```
GET /network/bandwidth
```

> **Note:** Bandwidth monitoring requires metrics integration and returns 501 until wired.

---

## Examples

See the [examples/network/](examples/network/) directory for:
- Multi-network VM setup
- Bandwidth monitoring scripts
- Traffic analysis automation
- Cilium policy templates
- Service mesh integration
