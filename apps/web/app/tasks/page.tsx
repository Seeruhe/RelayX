import { ConsoleCard } from '../_components/ConsoleCard';

export default function Page() {
  return (
    <section>
      <h2>Tasks</h2>
      <ConsoleCard title="P0 task lane" eyebrow="Operator queue">
        <p>Track deployment compile, runner apply, health observation, subscription token rotation, and rollback operations from the local runbook.</p>
        <ul>
          <li>Compile task: profile + node to DeploymentPlan.</li>
          <li>Runner task: signed command polling and result submission.</li>
          <li>Recovery task: rollback pointer to signed rollback command.</li>
        </ul>
      </ConsoleCard>
    </section>
  );
}
