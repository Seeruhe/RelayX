import { ConsoleCard } from '../_components/ConsoleCard';
import { CommandBlock } from '../_components/CommandBlock';

export default function Page() {
  return (
    <section>
      <h2>Settings</h2>
      <div className="grid two">
        <ConsoleCard title="Runner trust" eyebrow="Keys and tokens">
          <p>Register or rotate runner result public keys per node and run the runner with the matching signing key.</p>
          <CommandBlock
            title="Rotate runner result key"
            command={`curl -sS -X POST http://127.0.0.1:8080/nodes/{node_id}/runner-result-key/rotate \
  -H 'content-type: application/json' \
  -d '{"runner_result_public_key_hex":"<new-ed25519-public-key-hex>"}'`}
          />
          <CommandBlock title="Runner env" command={`RUNNER_API_TOKEN=dev-runner-token \
RUNNER_RESULT_SIGNING_KEY_HEX=<ed25519-secret-key-hex> \
CONTROL_PLANE_BASE_URL=http://127.0.0.1:8080 cargo run -p runner`} />
        </ConsoleCard>
        <ConsoleCard title="Subscription tokens" eyebrow="Rotation">
          <CommandBlock title="Issue token" command="curl -sS -X POST http://127.0.0.1:8080/subscriptions/{profile_id}/tokens" />
          <CommandBlock title="Rotate token" command="curl -sS -X POST http://127.0.0.1:8080/subscriptions/{profile_id}/tokens/rotate" />
        </ConsoleCard>
      </div>
    </section>
  );
}
