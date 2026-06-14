import { ConsoleCard } from '../_components/ConsoleCard';
import { CommandBlock } from '../_components/CommandBlock';

const stages = [
  ['Control-plane', 'Start the Axum API and use the local tenant admin flow.'],
  ['Runner', 'Register one node, send heartbeat, then poll signed commands.'],
  ['Deploy', 'Compile a Profile IR into a content-addressed Xray artifact.'],
  ['Observe', 'Read deployment health, usage rollups, quota, and expiry decisions.'],
  ['Subscribe', 'Serve grouped subscriptions with token rotation and policy filtering.'],
  ['Rollback', 'Queue signed rollback commands from stored rollback pointers.'],
];

export default function Page() {
  return (
    <section>
      <h2>Dashboard</h2>
      <p className="lede">P0 operator console for the local Xray control-plane runbook.</p>
      <div className="grid">
        {stages.map(([title, body]) => (
          <ConsoleCard key={title} title={title} eyebrow="P0 flow">
            <p>{body}</p>
          </ConsoleCard>
        ))}
      </div>
      <CommandBlock
        title="Local verification"
        command={`cargo fmt --check && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo test --workspace`}
      />
    </section>
  );
}
