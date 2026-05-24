import React, { useEffect, useId, useRef } from 'react';
import { ExternalLink, Info, X } from 'lucide-react';
import {
  ZYVOR_CONTACT_URL,
  ZYVOR_COPY,
  ZYVOR_DOCS_URL,
  ZYVOR_PRIVACY_URL,
  ZYVOR_PRODUCTS_URL,
  ZYVOR_TERMS_URL,
  ZYVOR_URL,
} from './ZyvorBrand';

export type ZyvorAboutLink = {
  label: string;
  href: string;
  description?: string;
};

export type ZyvorAboutModalProps = {
  product: string;
  productTagline?: string;
  version?: string;
  onClose: () => void;
  extraLinks?: ZyvorAboutLink[];
};

const ORANGE = '#f97316';

const defaultHelpLinks: ZyvorAboutLink[] = [
  {
    label: 'zyvor.dev',
    href: ZYVOR_URL,
    description: 'Zyvor platform home',
  },
  {
    label: 'Documentation & help',
    href: ZYVOR_DOCS_URL,
    description: 'Suite guides and getting started',
  },
  {
    label: 'Product suite',
    href: ZYVOR_PRODUCTS_URL,
    description: 'Compare Zyvor products',
  },
  {
    label: 'Contact',
    href: ZYVOR_CONTACT_URL,
    description: 'Support and enterprise inquiries',
  },
];

export function ZyvorAboutModal({
  product,
  productTagline = 'Part of the Zyvor platform suite.',
  version,
  onClose,
  extraLinks = [],
}: ZyvorAboutModalProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const helpLinks = [...defaultHelpLinks, ...extraLinks];

  useEffect(() => {
    panelRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-[1000] flex items-start justify-center bg-black/45 px-4 pt-[8vh] pb-8"
      role="presentation"
      onClick={onClose}
    >
      <div
        ref={panelRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="w-full max-w-lg overflow-hidden rounded-xl border border-slate-200 bg-white shadow-2xl outline-none"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4">
          <div className="flex items-center gap-2">
            <Info className="h-5 w-5 text-orange-500" aria-hidden />
            <h2 id={titleId} className="text-lg font-semibold text-slate-900">
              About {product}
            </h2>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-md p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-700"
            aria-label="Close about dialog"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="max-h-[min(62vh,520px)] overflow-y-auto px-5 py-4 text-sm text-slate-600">
          <p className="leading-relaxed">{productTagline}</p>
          <p className="mt-3 leading-relaxed">
            Built by{' '}
            <a
              href={ZYVOR_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="font-medium text-orange-500 hover:text-orange-400"
            >
              Zyvor
            </a>
            . Visit{' '}
            <a
              href={ZYVOR_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="font-medium text-orange-500 hover:text-orange-400"
            >
              zyvor.dev
            </a>{' '}
            for platform documentation, product guides, and support.
          </p>

          {version ? (
            <div className="mt-4 flex items-center justify-between rounded-lg border border-slate-200 bg-slate-50 px-3 py-2">
              <span className="text-slate-500">Version</span>
              <span className="font-mono text-slate-800">{version}</span>
            </div>
          ) : null}

          <h3 className="mt-5 mb-2 text-xs font-semibold uppercase tracking-wide text-slate-500">
            Help &amp; resources
          </h3>
          <ul className="space-y-2">
            {helpLinks.map((link) => {
              const external = /^https?:\/\//i.test(link.href);
              return (
              <li key={link.href + link.label}>
                <a
                  href={link.href}
                  {...(external
                    ? { target: '_blank', rel: 'noopener noreferrer' }
                    : {})}
                  className="group flex items-start justify-between gap-3 rounded-lg border border-transparent px-2 py-2 hover:border-slate-200 hover:bg-slate-50"
                >
                  <span>
                    <span className="flex items-center gap-1.5 font-medium text-slate-800 group-hover:text-orange-600">
                      {link.label}
                      {external ? (
                        <ExternalLink className="h-3.5 w-3.5 opacity-60" aria-hidden />
                      ) : null}
                    </span>
                    {link.description ? (
                      <span className="mt-0.5 block text-xs text-slate-500">{link.description}</span>
                    ) : null}
                  </span>
                </a>
              </li>
            );
            })}
          </ul>
        </div>

        <div className="border-t border-slate-200 bg-slate-50 px-5 py-4">
          <p className="text-center text-xs text-slate-500">
            <span style={{ color: ORANGE, fontWeight: 500 }}>{ZYVOR_COPY}</span>
            {' · '}
            Zyvor. All rights reserved.
          </p>
          <div className="mt-2 flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-xs">
            <a
              href={ZYVOR_PRIVACY_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="text-slate-500 hover:text-orange-500"
            >
              Privacy Policy
            </a>
            <a
              href={ZYVOR_TERMS_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="text-slate-500 hover:text-orange-500"
            >
              Terms of Service
            </a>
            <a
              href={ZYVOR_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="text-slate-500 hover:text-orange-500"
            >
              zyvor.dev
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ZyvorAboutModal;
