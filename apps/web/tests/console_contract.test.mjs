import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const appDir = new URL('../app/', import.meta.url);
const read = (path) => readFileSync(new URL(path, appDir), 'utf8');

test('P0 console pages are operational instead of placeholders', () => {
  const pages = ['dashboard', 'nodes', 'clients', 'profiles', 'deployments', 'tasks', 'logs', 'settings'];
  for (const page of pages) {
    const source = read(`${page}/page.tsx`);
    assert(!source.includes('placeholder'), `${page} still contains placeholder copy`);
    assert(!source.includes('The verified backend API is available'), `${page} still defers to backend-only flow`);
  }
});

test('dashboard surfaces the core P0 runbook and status cards', () => {
  const source = read('dashboard/page.tsx');
  for (const text of ['Control-plane', 'Runner', 'Deploy', 'Observe', 'Subscribe', 'Rollback']) {
    assert(source.includes(text), `dashboard missing ${text}`);
  }
});

test('operator pages include concrete control-plane API commands', () => {
  const expectations = {
    'nodes/page.tsx': ['/nodes/register', '/runner/nodes/{node_id}/heartbeat'],
    'profiles/page.tsx': ['/profiles/vless-reality', '/profiles/shadowsocks', '/profiles/trojan'],
    'clients/page.tsx': ['/clients', 'quota_bytes', 'expires_at', '/clients/{client_id}/quota', '/clients/{client_id}/expiry', 'bucket=hour', 'bucket=day', 'bucket=month'],
    'deployments/page.tsx': ['/deployments/compile', 'idempotency-key', '/deployments/{deployment_id}/rollback', '/deployments/{deployment_id}/health', '/deployments/{deployment_id}/readiness', '/deployments/{deployment_id}/advance', '/runner/nodes/{node_id}/deployments/{deployment_id}/health'],
    'settings/page.tsx': ['/nodes/{node_id}/runner-result-key/rotate', 'RUNNER_RESULT_SIGNING_KEY_HEX', '/subscriptions/{profile_id}/tokens/rotate'],
  };
  for (const [file, snippets] of Object.entries(expectations)) {
    const source = read(file);
    for (const snippet of snippets) {
      assert(source.includes(snippet), `${file} missing ${snippet}`);
    }
  }
});

test('shared console components are used by pages', () => {
  const componentFiles = readdirSync(new URL('_components/', appDir));
  assert(componentFiles.includes('ConsoleCard.tsx'));
  assert(componentFiles.includes('CommandBlock.tsx'));
  const pagesSource = ['dashboard', 'nodes', 'clients', 'profiles', 'deployments']
    .map((page) => read(`${page}/page.tsx`))
    .join('\n');
  assert(pagesSource.includes('ConsoleCard'));
  assert(pagesSource.includes('CommandBlock'));
});
