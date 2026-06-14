import { CommandBlock } from '../_components/CommandBlock';
import { ConsoleCard } from '../_components/ConsoleCard';

export default function Page() {
  return (
    <section>
      <h2>Clients</h2>
      <div className="grid two">
        <ConsoleCard title="Create credential" eyebrow="Client credential">
          <p>Client creation supports <code>quota_bytes</code> and <code>expires_at</code>; subscriptions hide revoked, expired, or over-quota credentials.</p>
          <CommandBlock
            title="POST /clients"
            command={`curl -sS -X POST http://127.0.0.1:8080/clients \
  -H 'content-type: application/json' \
  -d '{"client_id":"client-a","profile_id":"profile-a","display_name":"Alice","uuid":"2f4f6f8a-1111-4c4c-9999-111111111111","quota_bytes":1000000000,"expires_at":"2026-12-31T00:00:00Z"}'`}
          />
        </ConsoleCard>
        <ConsoleCard title="Policy decisions" eyebrow="Quota and expiry">
          <p>Usage rollups are available at hourly, daily, and monthly buckets; quota decisions use the hourly accumulated rollup.</p>
          <CommandBlock
            title="GET /clients/{client_id}/quota"
            command="curl -sS http://127.0.0.1:8080/clients/{client_id}/quota"
          />
          <CommandBlock
            title="GET /clients/{client_id}/expiry"
            command="curl -sS http://127.0.0.1:8080/clients/{client_id}/expiry"
          />
          <CommandBlock
            title="GET /usage/credentials/{client_id}/rollups/latest"
            command={`curl -sS 'http://127.0.0.1:8080/usage/credentials/{client_id}/rollups/latest?bucket=hour'
curl -sS 'http://127.0.0.1:8080/usage/credentials/{client_id}/rollups/latest?bucket=day'
curl -sS 'http://127.0.0.1:8080/usage/credentials/{client_id}/rollups/latest?bucket=month'`}
          />
        </ConsoleCard>
      </div>
    </section>
  );
}
