import { CommandBlock } from '../_components/CommandBlock';
import { ConsoleCard } from '../_components/ConsoleCard';

export default function Page() {
  return (
    <section>
      <h2>Profiles</h2>
      <div className="grid">
        <ConsoleCard title="VLESS + REALITY" eyebrow="Profile IR">
          <CommandBlock
            title="POST /profiles/vless-reality"
            command={`curl -sS -X POST http://127.0.0.1:8080/profiles/vless-reality \
  -H 'content-type: application/json' \
  -d '{"profile_id":"profile-a","server_name":"example.com"}'`}
          />
        </ConsoleCard>
        <ConsoleCard title="Shadowsocks" eyebrow="Profile IR">
          <CommandBlock
            title="POST /profiles/shadowsocks"
            command={`curl -sS -X POST http://127.0.0.1:8080/profiles/shadowsocks \
  -H 'content-type: application/json' \
  -d '{"profile_id":"profile-ss","port":8388}'`}
          />
        </ConsoleCard>
        <ConsoleCard title="Trojan" eyebrow="Profile IR">
          <CommandBlock
            title="POST /profiles/trojan"
            command={`curl -sS -X POST http://127.0.0.1:8080/profiles/trojan \
  -H 'content-type: application/json' \
  -d '{"profile_id":"profile-trojan","server_name":"trojan.example.com"}'`}
          />
        </ConsoleCard>
      </div>
    </section>
  );
}
