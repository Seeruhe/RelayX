import { ConsoleCard } from '../_components/ConsoleCard';
import { CommandBlock } from '../_components/CommandBlock';

export default function Page() {
  return (
    <section>
      <h2>Logs</h2>
      <ConsoleCard title="Audit and evidence" eyebrow="Append-only records">
        <p>Use the backend evidence endpoints to inspect artifacts, rollback pointers, snapshots, and health records.</p>
        <CommandBlock title="Artifact bytes" command="curl -sS http://127.0.0.1:8080/artifacts/{artifact_id}/bytes" />
        <CommandBlock title="Deployment snapshot" command="curl -sS http://127.0.0.1:8080/deployments/{deployment_id}/snapshot" />
      </ConsoleCard>
    </section>
  );
}
