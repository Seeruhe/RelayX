import type { ReactNode } from 'react';

type ConsoleCardProps = {
  title: string;
  eyebrow?: string;
  children: ReactNode;
};

export function ConsoleCard({ title, eyebrow, children }: ConsoleCardProps) {
  return (
    <section className="card console-card">
      {eyebrow ? <p className="eyebrow">{eyebrow}</p> : null}
      <h3>{title}</h3>
      <div>{children}</div>
    </section>
  );
}
