import './styles.css';
import type { ReactNode } from 'react';

const nav = ['dashboard', 'nodes', 'clients', 'profiles', 'deployments', 'tasks', 'logs', 'settings'];

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <aside>
          <h1>Proxy Control</h1>
          {nav.map((item) => <a key={item} href={`/${item}`}>{item}</a>)}
        </aside>
        <main>{children}</main>
      </body>
    </html>
  );
}
