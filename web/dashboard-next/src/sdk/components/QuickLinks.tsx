import React from 'react';

interface QuickLink {
  title: string;
  description: string;
  icon: string;
  href: string;
  onClick?: () => void;
}

interface QuickLinksProps {
  links: QuickLink[];
}

export const QuickLinks: React.FC<QuickLinksProps> = ({ links }) => {
  return (
    <div style={{
      backgroundColor: '#f0f2f7',
      padding: '24px 16px',
    }}>
      <div style={{
        maxWidth: '1400px',
        margin: '0 auto',
      }}>
        <h2 style={{
          margin: '0 0 12px 0',
          fontSize: '18px',
          fontWeight: '600',
          color: '#000',
        }}>
          Quick links
        </h2>

        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
          gap: '12px',
        }}>
          {links.map((link, index) => (
            <a
              key={index}
              href={link.href}
              onClick={(e) => {
                if (link.onClick) {
                  e.preventDefault();
                  link.onClick();
                }
              }}
              style={{
                backgroundColor: '#fff',
                border: '1px solid #e0e0e0',
                borderRadius: '3px',
                padding: '12px',
                textDecoration: 'none',
                transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
                display: 'block',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = '#f0583a';
                e.currentTarget.style.transform = 'translateY(-2px)';
                e.currentTarget.style.boxShadow = '0 4px 8px rgba(0, 0, 0, 0.08)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = '#e0e0e0';
                e.currentTarget.style.transform = 'translateY(0)';
                e.currentTarget.style.boxShadow = 'none';
              }}
            >
              <div style={{
                fontSize: '20px',
                marginBottom: '8px',
              }}>
                {link.icon}
              </div>
              <h3 style={{
                margin: '0 0 4px 0',
                fontSize: '11px',
                fontWeight: '600',
                color: '#000',
              }}>
                {link.title}
              </h3>
              <p style={{
                margin: 0,
                fontSize: '9px',
                color: '#6b7280',
                lineHeight: 1.4,
              }}>
                {link.description}
              </p>
            </a>
          ))}
        </div>
      </div>
    </div>
  );
};
