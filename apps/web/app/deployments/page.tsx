import { CommandBlock } from '../_components/CommandBlock';
import { ConsoleCard } from '../_components/ConsoleCard';

export default function Page() {
  return (
    <section>
      <h2>Deployments</h2>
      <div className="grid two">
        <ConsoleCard title="Compile and enqueue" eyebrow="DeploymentPlan">
          <p>Send <code>idempotency-key</code> so retries replay the first response without duplicate runner commands.</p>
          <CommandBlock
            title="POST /deployments/compile"
            command={`curl -sS -X POST http://127.0.0.1:8080/deployments/compile \
  -H 'content-type: application/json' \
  -H 'idempotency-key: compile-profile-a-node-a-1' \
  -d '{"profile_id":"profile-a","node_id":"node-a"}'`}
          />
        </ConsoleCard>
        <ConsoleCard title="Observe and rollback" eyebrow="Safety controls">
          <p>Advance evaluates readiness: ready deployments are promoted; latest unhealthy deployments queue rollback from the stored rollback pointer.</p>
          <CommandBlock title="GET /deployments/{deployment_id}/health" command="curl -sS http://127.0.0.1:8080/deployments/{deployment_id}/health" />
          <CommandBlock title="GET /deployments/{deployment_id}/readiness" command="curl -sS http://127.0.0.1:8080/deployments/{deployment_id}/readiness" />
          <CommandBlock title="POST /deployments/{deployment_id}/advance" command="curl -sS -X POST http://127.0.0.1:8080/deployments/{deployment_id}/advance" />
          <CommandBlock
            title="POST /runner/nodes/{node_id}/deployments/{deployment_id}/health"
            command={`curl -sS -X POST http://127.0.0.1:8080/runner/nodes/{node_id}/deployments/{deployment_id}/health \
  -H 'content-type: application/json' \
  -H 'x-runner-token: dev-runner-token' \
  -d '{"status":"healthy","payload_json":{"probe":"subscription_fetch_ok"}}'`}
          />
          <CommandBlock title="POST /deployments/{deployment_id}/rollback" command="curl -sS -X POST http://127.0.0.1:8080/deployments/{deployment_id}/rollback" />
        </ConsoleCard>
      </div>
    </section>
  );
}
