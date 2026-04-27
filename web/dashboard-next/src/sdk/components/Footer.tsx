import React from 'react';

export const Footer: React.FC = () => {
  return (
    <footer style={{
      backgroundColor: '#222324',
      color: '#fff',
      marginTop: '64px',
    }}>
      <div style={{
        maxWidth: '1400px',
        margin: '0 auto',
        padding: '48px 24px 24px',
      }}>
        {/* Footer Columns */}
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
          gap: '48px',
          marginBottom: '48px',
        }}>
          {/* About Column */}
          <div>
            <h3 style={{
              fontSize: '14px',
              fontWeight: '600',
              marginBottom: '16px',
            }}>
              about VMRogue
            </h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {["Dashboard", "REST API /api/v1", "KubeVirt CRDs", "Build from source"].map((item) => (
                <li key={item} style={{ marginBottom: '12px' }}>
                  <a
                    href="#"
                    style={{
                      color: '#fff',
                      fontSize: '14px',
                      textDecoration: 'none',
                      opacity: 0.8,
                      transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.color = '#f0583a';
                      e.currentTarget.style.opacity = '1';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.color = '#fff';
                      e.currentTarget.style.opacity = '0.8';
                    }}
                  >
                    {item}
                  </a>
                </li>
              ))}
            </ul>
          </div>

          {/* Services Column */}
          <div>
            <h3 style={{
              fontSize: '14px',
              fontWeight: '600',
              marginBottom: '16px',
            }}>
              kubevirt
            </h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {["VirtualMachines", "VMI", "Migrations", "Snapshots"].map((item) => (
                <li key={item} style={{ marginBottom: '12px' }}>
                  <a
                    href="#"
                    style={{
                      color: '#fff',
                      fontSize: '14px',
                      textDecoration: 'none',
                      opacity: 0.8,
                      transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.color = '#f0583a';
                      e.currentTarget.style.opacity = '1';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.color = '#fff';
                      e.currentTarget.style.opacity = '0.8';
                    }}
                  >
                    {item}
                  </a>
                </li>
              ))}
            </ul>
          </div>

          {/* Support Column */}
          <div>
            <h3 style={{
              fontSize: '14px',
              fontWeight: '600',
              marginBottom: '16px',
            }}>
              cluster ops
            </h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {["Namespaces", "Nodes", "Storage classes", "Network policies"].map((item) => (
                <li key={item} style={{ marginBottom: '12px' }}>
                  <a
                    href="#"
                    style={{
                      color: '#fff',
                      fontSize: '14px',
                      textDecoration: 'none',
                      opacity: 0.8,
                      transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.color = '#f0583a';
                      e.currentTarget.style.opacity = '1';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.color = '#fff';
                      e.currentTarget.style.opacity = '0.8';
                    }}
                  >
                    {item}
                  </a>
                </li>
              ))}
            </ul>
          </div>

          {/* Contact Column */}
          <div>
            <h3 style={{
              fontSize: '14px',
              fontWeight: '600',
              marginBottom: '16px',
            }}>
              quick links
            </h3>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              {["Health", "VM list", "Alerts", "Metrics WS"].map((item) => (
                <li key={item} style={{ marginBottom: '12px' }}>
                  <a
                    href="#"
                    style={{
                      color: '#fff',
                      fontSize: '14px',
                      textDecoration: 'none',
                      opacity: 0.8,
                      transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.color = '#f0583a';
                      e.currentTarget.style.opacity = '1';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.color = '#fff';
                      e.currentTarget.style.opacity = '0.8';
                    }}
                  >
                    {item}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        </div>

        {/* Bottom Bar */}
        <div style={{
          borderTop: '1px solid rgba(255, 255, 255, 0.1)',
          paddingTop: '24px',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          flexWrap: 'wrap',
          gap: '16px',
        }}>
          <p style={{
            margin: 0,
            fontSize: '12px',
            opacity: 0.6,
          }}>
            © {new Date().getFullYear()} VMRogue. UI layout from HyperSDK dashboard-react; data from the VMRogue API.
          </p>
          <div style={{ display: 'flex', gap: '24px' }}>
            <a
              href="#"
              style={{
                color: '#fff',
                fontSize: '12px',
                textDecoration: 'none',
                opacity: 0.6,
                transition: 'opacity 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
              }}
              onMouseEnter={(e) => e.currentTarget.style.opacity = '1'}
              onMouseLeave={(e) => e.currentTarget.style.opacity = '0.6'}
            >
              Privacy Policy
            </a>
            <a
              href="#"
              style={{
                color: '#fff',
                fontSize: '12px',
                textDecoration: 'none',
                opacity: 0.6,
                transition: 'opacity 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
              }}
              onMouseEnter={(e) => e.currentTarget.style.opacity = '1'}
              onMouseLeave={(e) => e.currentTarget.style.opacity = '0.6'}
            >
              Terms of Service
            </a>
            <a
              href="#"
              style={{
                color: '#fff',
                fontSize: '12px',
                textDecoration: 'none',
                opacity: 0.6,
                transition: 'opacity 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
              }}
              onMouseEnter={(e) => e.currentTarget.style.opacity = '1'}
              onMouseLeave={(e) => e.currentTarget.style.opacity = '0.6'}
            >
              Accessibility
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
};
