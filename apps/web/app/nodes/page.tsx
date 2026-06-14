import { CommandBlock } from '../_components/CommandBlock';
import { ConsoleCard } from '../_components/ConsoleCard';

export default function Page() {
  return (
    <section>
      <h2>Nodes</h2>
      <div className="grid two">
        <ConsoleCard title="Register runner node" eyebrow="Node registration">
          <p>Use <code>/nodes/register</code> with the one-time registration token and optional runner result public key.</p>
          <CommandBlock
            title="POST /nodes/register"
            command={`curl -sS -X POST http://127.0.0.1:8080/nodes/register \
  -H 'content-type: application/json' \
  -d '{"registration_token":"dev-registration-token","node_id":"node-a","xray_version":"1.8.8"}'`}
          />
        </ConsoleCard>
        <ConsoleCard title="Heartbeat and capability snapshot" eyebrow="Runner telemetry">
          <p>Runner API calls require <code>X-Runner-Token</code>.</p>
          <CommandBlock
            title="POST /runner/nodes/{node_id}/heartbeat"
            command={`curl -sS -X POST http://127.0.0.1:8080/runner/nodes/{node_id}/heartbeat \
  -H 'content-type: application/json' \
  -H 'x-runner-token: dev-runner-token' \
  -d '{"capability_snapshot":{"xray_version":"1.8.8","os":"linux"}}'`}
          />
        </ConsoleCard>
      </div>
    </section>
  );
}
