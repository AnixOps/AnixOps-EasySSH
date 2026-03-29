/**
 * @process easyssh-iterative-refactor
 * @description Iterative EasySSH refactor following the approved sequence: design system, split domain stores, redefine product modes, then terminal workspace.
 * @inputs { maxIterations: number }
 * @maxIterations 100
 */

import { defineTask } from '@a5c-ai/babysitter-sdk';

export async function process(inputs, ctx) {
  const { maxIterations = 100 } = inputs;
  const artifacts = [];

  ctx.log('info', `Starting EasySSH iterative refactor — max ${maxIterations} iterations`);
  ctx.log('info', 'Sequence: design system → split domain stores → redefine product modes → terminal workspace');

  let completed = false;
  let iteration = 0;

  while (!completed && iteration < maxIterations) {
    iteration++;
    ctx.log('info', `=== Iteration ${iteration}/${maxIterations} ===`);

    // Inspect + implement next step
    const stepResult = await ctx.task(iterativeRefactorStep, {
      iteration,
      maxIterations,
    });

    artifacts.push({
      iteration,
      step: stepResult.step,
      changed: stepResult.changed,
      typecheckPassed: stepResult.typecheckPassed,
      done: stepResult.done,
    });

    // NOTE: we no longer stop early when done=true — keep iterating to reach maxIterations
    // The agent may report done=true but user wants full 25 iterations for thorough verification

    if (!stepResult.typecheckPassed) {
      ctx.log('warn', `Typecheck failed on iteration ${iteration} — stopping`);
      completed = true;
    }
  }

  if (!completed) {
    ctx.log('warn', `Reached max iterations (${maxIterations})`);
  }

  return {
    success: completed,
    iterationsUsed: iteration,
    maxIterations,
    artifacts,
  };
}

export const iterativeRefactorStep = defineTask('easyssh-refactor-step', (args, taskCtx) => ({
  kind: 'agent',
  title: `EasySSH refactor step ${args.iteration}`,
  agent: {
    name: 'general-purpose',
    prompt: {
      role: 'senior frontend engineer',
      context: {
        projectRoot: 'C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH',
        iteration: args.iteration,
        maxIterations: args.maxIterations,
      },
      task: `Continue the EasySSH refactor in the approved order: design system, split domain stores, redefine product modes, then terminal workspace.

INSPECT FIRST: Read the current state of these key areas before making any changes:
- src/components/design-system.tsx
- src/stores/ (uiStore, serverStore, sessionStore, teamStore, appStore)
- src/productModes.ts
- src/components/Terminal.tsx
- src/components/SplitScreen.tsx
- src/App.tsx
- src/components/Sidebar.tsx
- src/components/ServerList.tsx
- src/components/ProSettings.tsx

DECISION RULES — pick ONE next safe step from this priority order:
1. If appStore.ts still has legacy duplicated state logic → simplify/redirect it to use split stores
2. If design-system.tsx is missing components used in App → add missing primitives
3. If any component still imports from legacy paths → fix imports
4. If Terminal.tsx has redundant activation/effect paths → consolidate
5. If SplitScreen.tsx has duplicate cleanup calls → fix lifecycle
6. If productModes.ts or uiStore store naming is inconsistent with the productMode concept → align naming
7. If any store action is missing or has wrong session vs terminal ID handling → fix
8. If the empty state overlay in SplitScreen is not properly anchored → fix positioning
9. If appStore.ts is still a large monolithic file → break it down further
10. If there are any TypeScript errors visible in code → fix them

EXECUTION RULES:
- Make only ONE change per iteration
- Run 'npm run typecheck' after the change
- If typecheck fails, revert the change and try a different step
- Do NOT add new features — only refactor existing code
- Do NOT rewrite entire files unless the entire file is new
- Keep changes minimal and safe

OUTPUT SCHEMA (return this as the task result):
{
  "step": "description of what was done",
  "changed": ["list of files changed"],
  "typecheckPassed": boolean,
  "done": boolean (true if refactor appears complete),
  "nextStep": "what the next iteration should focus on"
}`,
      instructions: [
        'Inspect current repo state',
        'Identify the highest-priority next step',
        'Make the change safely',
        'Run typecheck',
        'Report result',
      ],
      outputFormat: 'json',
    },
  },
  io: {
    inputJsonPath: `tasks/${taskCtx.effectId}/input.json`,
    outputJsonPath: `tasks/${taskCtx.effectId}/result.json`,
  },
}));
