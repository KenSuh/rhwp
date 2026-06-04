// rhwp 기준 표 조작 QA 매트릭스를 고정하는 실행형 테스트.
import assert from 'node:assert/strict';
import test from 'node:test';

type Priority = 'P0' | 'P1' | 'P2';

interface TableInteractionScenario {
  readonly id: string;
  readonly priority: Priority;
  readonly fixture: string;
  readonly area: 'selection' | 'context-menu' | 'cut-delete' | 'paste' | 'nested-table' | 'pagination';
  readonly userAction: string;
  readonly expectedRhwpBehavior: string;
  readonly automationHook: string;
}

const scenarios: readonly TableInteractionScenario[] = [
  {
    id: 'TBL-SEL-001',
    priority: 'P0',
    fixture: '/sample.hwpx',
    area: 'selection',
    userAction: '표 셀 안의 본문 텍스트에서 드래그를 시작하고 셀 경계 안에 머문다.',
    expectedRhwpBehavior: '시작 위치 기준 텍스트만 하이라이트되고 셀 자체 선택으로 전환되지 않는다.',
    automationHook: 'text-drag-in-cell',
  },
  {
    id: 'TBL-SEL-002',
    priority: 'P0',
    fixture: '/sample.hwpx',
    area: 'selection',
    userAction: '텍스트 드래그 중 셀 경계를 넘어 다음 셀 영역까지 이동한다.',
    expectedRhwpBehavior: '기존 텍스트 하이라이트는 사라지고, 시작 셀부터 현재 마우스 위치의 셀까지 셀 선택으로 전환된다.',
    automationHook: 'text-drag-cross-cell-boundary',
  },
  {
    id: 'TBL-SEL-003',
    priority: 'P0',
    fixture: '/sample.hwpx',
    area: 'selection',
    userAction: '셀 선택 드래그 중 아래 셀로 갔다가 다시 위 셀로 돌아온다.',
    expectedRhwpBehavior: '현재 마우스 위치 기준 선택 범위가 자연스럽게 더해지고 빠지며, 이전에 지나간 셀이 고정 선택으로 남지 않는다.',
    automationHook: 'cell-drag-live-range',
  },
  {
    id: 'TBL-SEL-004',
    priority: 'P0',
    fixture: '/sample.hwpx',
    area: 'selection',
    userAction: '병합 셀이 포함된 행과 일반 셀이 섞인 표에서 여러 셀을 드래그 선택한다.',
    expectedRhwpBehavior: '마우스 시작 셀과 현재 셀의 논리 범위에 포함되는 셀만 선택되고, 다른 표나 다른 행으로 hitTest가 튀지 않는다.',
    automationHook: 'merged-cell-drag-range',
  },
  {
    id: 'TBL-MENU-001',
    priority: 'P1',
    fixture: '/sangsaeng-smartfactory-application.hwpx',
    area: 'context-menu',
    userAction: '여러 셀을 선택한 뒤 오른쪽 클릭한다.',
    expectedRhwpBehavior: '표 메뉴가 유지되고 셀 높이를 같게, 셀 너비를 같게가 선택 가능한 메뉴로 표시된다.',
    automationHook: 'multi-cell-context-menu',
  },
  {
    id: 'TBL-CUT-001',
    priority: 'P0',
    fixture: '/sangsaeng-smartfactory-application.hwpx',
    area: 'cut-delete',
    userAction: '왼쪽 끝부터 오른쪽 끝까지 행 전체를 관통하는 셀 선택 후 잘라내기 또는 지우기를 실행한다.',
    expectedRhwpBehavior: '내용만 지우고 셀 모양을 남길지, 셀 자체를 지울지 선택하는 확인 다이얼로그가 표시된다.',
    automationHook: 'full-row-cell-cut-delete-choice',
  },
  {
    id: 'TBL-CUT-002',
    priority: 'P0',
    fixture: '/sangsaeng-smartfactory-application.hwpx',
    area: 'cut-delete',
    userAction: '행 전체를 관통하지 않는 일부 셀 선택 후 잘라내기 또는 지우기를 실행한다.',
    expectedRhwpBehavior: '확인 다이얼로그 없이 선택 셀의 내용만 지워지고 셀 구조와 표 모양은 유지된다.',
    automationHook: 'partial-cell-cut-delete-clear-content',
  },
  {
    id: 'TBL-PASTE-001',
    priority: 'P0',
    fixture: '/sample.hwpx',
    area: 'paste',
    userAction: '표 또는 표처럼 취급되는 개체를 잘라낸 뒤 다른 페이지의 커서 위치에 붙여넣는다.',
    expectedRhwpBehavior: '붙여넣기 결과가 원래 위치가 아니라 현재 커서 또는 현재 선택 셀 위치에 삽입된다.',
    automationHook: 'clipboard-target-caret-resolution',
  },
  {
    id: 'TBL-NEST-001',
    priority: 'P1',
    fixture: '/sample.hwpx',
    area: 'nested-table',
    userAction: '외부 표 셀 안에 들어 있는 내부 표의 테두리 선을 더블클릭한다.',
    expectedRhwpBehavior: '외부 표가 아니라 더블클릭한 내부 표 자체가 선택된다.',
    automationHook: 'nested-table-border-double-click',
  },
  {
    id: 'TBL-PAGE-001',
    priority: 'P0',
    fixture: '/sample.hwpx',
    area: 'pagination',
    userAction: '페이지 하단 경계에 걸린 표나 글자처럼 취급되는 표의 높이를 늘린다.',
    expectedRhwpBehavior: '다음 문단과 겹치지 않고 표가 다음 페이지로 자연스럽게 밀리거나 페이지 흐름이 재계산된다.',
    automationHook: 'inline-table-pagination-reflow',
  },
  {
    id: 'TBL-AI-001',
    priority: 'P1',
    fixture: '/sample.hwpx',
    area: 'paste',
    userAction: '표 안 또는 표 안의 표에서 AI 생성 결과를 삽입한다.',
    expectedRhwpBehavior: 'AI 결과물은 현재 표 컨텍스트를 벗어나지 않고 선택 셀 또는 내부 표 셀 안에 생성된다.',
    automationHook: 'ai-result-insert-table-context',
  },
];

test('table interaction matrix covers current rhwp parity blockers', () => {
  const ids = scenarios.map((scenario) => scenario.id);
  assert.equal(new Set(ids).size, scenarios.length, 'scenario ids must be unique');

  assert.deepEqual(
    ids.sort(),
    [
      'TBL-AI-001',
      'TBL-CUT-001',
      'TBL-CUT-002',
      'TBL-MENU-001',
      'TBL-NEST-001',
      'TBL-PAGE-001',
      'TBL-PASTE-001',
      'TBL-SEL-001',
      'TBL-SEL-002',
      'TBL-SEL-003',
      'TBL-SEL-004',
    ],
  );
});

test('table interaction matrix keeps every scenario executable against a demo fixture', () => {
  const allowedFixtures = new Set([
    '/sample.hwpx',
    '/sangsaeng-smartfactory-application.hwpx',
    '/seoul-root-auto-2026.hwpx',
  ]);

  for (const scenario of scenarios) {
    assert.match(scenario.id, /^TBL-[A-Z]+-\d{3}$/);
    assert.ok(allowedFixtures.has(scenario.fixture), `${scenario.id} uses an unknown fixture`);
    assert.ok(scenario.userAction.length > 20, `${scenario.id} is missing a concrete user action`);
    assert.ok(
      scenario.expectedRhwpBehavior.length > 20,
      `${scenario.id} is missing the expected rhwp behavior`,
    );
    assert.match(scenario.automationHook, /^[a-z0-9-]+$/);
  }
});

test('table interaction matrix keeps GA blocker priorities explicit', () => {
  const p0Ids = scenarios
    .filter((scenario) => scenario.priority === 'P0')
    .map((scenario) => scenario.id)
    .sort();

  assert.deepEqual(p0Ids, [
    'TBL-CUT-001',
    'TBL-CUT-002',
    'TBL-PAGE-001',
    'TBL-PASTE-001',
    'TBL-SEL-001',
    'TBL-SEL-002',
    'TBL-SEL-003',
    'TBL-SEL-004',
  ]);
});
