import React from 'react';

interface HeroProps {
  title: string;
  subtitle: string;
  onNewJob?: () => void;
}

export const Hero: React.FC<HeroProps> = ({ title, subtitle, onNewJob }) => {
  return (
    <div style={{
      backgroundColor: '#f0f2f7',
      padding: '40px 24px',
      textAlign: 'center',
      position: 'relative',
      overflow: 'hidden',
    }}>
      <div style={{
        maxWidth: '1400px',
        margin: '0 auto',
        position: 'relative',
        zIndex: 1,
      }}>
        <h1 style={{
          margin: '0 0 12px 0',
          fontSize: '32px',
          fontWeight: '700',
          color: '#f0583a',
          lineHeight: 1.2,
        }}>
          {title}
        </h1>
        <p style={{
          margin: '0 0 24px 0',
          fontSize: '16px',
          color: '#f0583a',
          maxWidth: '800px',
          marginLeft: 'auto',
          marginRight: 'auto',
        }}>
          {subtitle}
        </p>

        {onNewJob && (
          <button
            onClick={onNewJob}
            style={{
              padding: '10px 24px',
              backgroundColor: '#fff',
              color: '#222324',
              border: '2px solid #222324',
              borderRadius: '4px',
              fontSize: '14px',
              fontWeight: '600',
              cursor: 'pointer',
              transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = '#f0583a';
              e.currentTarget.style.borderColor = '#f0583a';
              e.currentTarget.style.color = '#fff';
              e.currentTarget.style.transform = 'translateY(-2px)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = '#fff';
              e.currentTarget.style.borderColor = '#222324';
              e.currentTarget.style.color = '#222324';
              e.currentTarget.style.transform = 'translateY(0)';
            }}
          >
            New export job
          </button>
        )}
      </div>
    </div>
  );
};
