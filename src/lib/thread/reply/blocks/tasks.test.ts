import { describe, expect, it } from 'vitest'
import { parseReply, replyProse, replySearchText } from '../parse'
import { draftRollup, type TaskDraft } from './tasks'

/** A reply's prose and the drafts its `<cormux-tasks>` block carries. */
function splitTaskCards(text: string, streaming = false) {
  const segments = parseReply(text, { streaming })
  const drafts = segments.flatMap((segment): TaskDraft[] =>
    segment.kind === 'tagBlock' && segment.block.kind === 'tasks'
      ? segment.block.data
      : [],
  )
  return { text: replyProse(segments), drafts }
}

describe('the tasks block', () => {
  it('leaves ordinary replies alone', () => {
    expect(splitTaskCards('Two tasks drafted above.')).toEqual({
      text: 'Two tasks drafted above.',
      drafts: [],
    })
  })

  it('parses drafts and removes the block from the prose', () => {
    const reply = [
      'Here are the drafts.',
      '',
      '<cormux-tasks>',
      '[{"title":"Add login rate limit","description":"Throttle after 5 attempts.","priority":"high","points":3,"labels":["auth","api"]},',
      ' {"title":"Fix typo","description":"recieve → receive"}]',
      '</cormux-tasks>',
    ].join('\n')
    expect(splitTaskCards(reply)).toEqual({
      text: 'Here are the drafts.',
      drafts: [
        {
          kind: 'task',
          title: 'Add login rate limit',
          description: 'Throttle after 5 attempts.',
          priority: 'high',
          points: 3,
          labels: ['auth', 'api'],
          acceptance: [],
          children: [],
        },
        {
          kind: 'task',
          title: 'Fix typo',
          description: 'recieve → receive',
          labels: [],
          acceptance: [],
          children: [],
        },
      ],
    })
  })

  it('tolerates a code fence', () => {
    const reply =
      'Drafts.\n<cormux-tasks>\n```json\n[{"title":"Ship it","points":5}]\n```\n</cormux-tasks>'
    expect(splitTaskCards(reply).drafts).toEqual([
      {
        kind: 'task',
        title: 'Ship it',
        points: 5,
        labels: [],
        acceptance: [],
        children: [],
      },
    ])
  })

  it('reads only the fields and shapes the skill writes', () => {
    const reply = `<cormux-tasks>${JSON.stringify([
      { name: 'No title' },
      {
        title: 'Aliases',
        kind: 'story',
        body: 'not a description',
        tags: ['x'],
        points: '3',
        acceptance: 'a string, not a list',
        tasks: [{ title: 'not a child' }],
      },
    ])}</cormux-tasks>`
    expect(splitTaskCards(reply).drafts).toEqual([
      {
        kind: 'task',
        title: 'Aliases',
        labels: [],
        acceptance: [],
        children: [],
      },
    ])
    expect(
      splitTaskCards('<cormux-tasks>{"title":"Not in a list"}</cormux-tasks>')
        .drafts,
    ).toEqual([])
  })

  it('drops drafts without a title and non-object entries', () => {
    const reply =
      '<cormux-tasks>[{"title":"  "},{"title":"Real"},42]</cormux-tasks>'
    expect(splitTaskCards(reply).drafts).toEqual([
      { kind: 'task', title: 'Real', labels: [], acceptance: [], children: [] },
    ])
  })

  it('hides a block that is still streaming, without drafts yet', () => {
    expect(
      splitTaskCards('Drafts so far.\n<cormux-tasks>\n[{"tit', true),
    ).toEqual({ text: 'Drafts so far.', drafts: [] })
    expect(splitTaskCards('Drafts so far.\n<cormux-ta', true)).toEqual({
      text: 'Drafts so far.',
      drafts: [],
    })
  })

  it('shows a block the finished reply never closed, rather than dropping the rest', () => {
    const reply = 'Drafts.\n<cormux-tasks>\n[{"title":"A"}]\nThat is all.'
    expect(splitTaskCards(reply)).toEqual({ text: reply, drafts: [] })
  })

  it('searches the drafts as they read, not their JSON', () => {
    const text = replySearchText(
      parseReply(
        'Done.<cormux-tasks>[{"title":"Add rate limit","acceptance":["429 after 5"]}]</cormux-tasks>',
        { streaming: false },
      ),
    )
    expect(text).toContain('Add rate limit')
    expect(text).toContain('429 after 5')
    expect(text).not.toContain('acceptance')
  })

  it('keeps a complete but invalid block visible rather than losing it', () => {
    const reply = 'Drafts.\n<cormux-tasks>not json</cormux-tasks>'
    expect(splitTaskCards(reply)).toEqual({ text: reply, drafts: [] })
  })

  it('keeps text after the closing tag', () => {
    expect(
      splitTaskCards(
        'Done.<cormux-tasks>[{"title":"A"}]</cormux-tasks>\nShall I create these?',
      ),
    ).toEqual({
      text: 'Done.\nShall I create these?',
      drafts: [
        { kind: 'task', title: 'A', labels: [], acceptance: [], children: [] },
      ],
    })
  })

  it('parses an epic scoped before its tasks and subtasks', () => {
    const reply = [
      'Scoped.',
      '<cormux-tasks>',
      JSON.stringify([
        {
          kind: 'epic',
          title: 'Self-serve onboarding',
          outcome: 'New teams finish setup without support.',
          scope: { in: ['Invite flow'], out: ['SSO'] },
          acceptance: ['Median setup under 10 minutes'],
          children: [
            {
              title: 'Invite teammates',
              points: 5,
              acceptance: ['Invite link expires in 7 days'],
              children: [
                { title: 'Add invite tokens', points: 2 },
                { title: 'Send invite email' },
              ],
            },
          ],
        },
      ]),
      '</cormux-tasks>',
    ].join('\n')
    const { drafts } = splitTaskCards(reply)
    expect(drafts).toHaveLength(1)
    expect(drafts[0]).toMatchObject({
      kind: 'epic',
      outcome: 'New teams finish setup without support.',
      scope: { in: ['Invite flow'], out: ['SSO'] },
      acceptance: ['Median setup under 10 minutes'],
    })
    expect(drafts[0]?.children[0]).toMatchObject({
      kind: 'task',
      title: 'Invite teammates',
      acceptance: ['Invite link expires in 7 days'],
    })
    expect(
      drafts[0]?.children[0]?.children.map((child) => [
        child.kind,
        child.title,
      ]),
    ).toEqual([
      ['subtask', 'Add invite tokens'],
      ['subtask', 'Send invite email'],
    ])
  })

  it('rolls up counts and leaf estimates across the hierarchy', () => {
    const { drafts } = splitTaskCards(
      '<cormux-tasks>[{"kind":"epic","title":"E","children":[' +
        '{"title":"T1","children":[{"title":"S1","points":2},{"title":"S2"}]},' +
        '{"title":"T2","points":3}]}]</cormux-tasks>',
    )
    expect(draftRollup(drafts)).toEqual({
      epics: 1,
      tasks: 2,
      subtasks: 2,
      points: 5,
      unestimatedLeaves: 1,
    })
    expect(draftRollup([])).toEqual({
      epics: 0,
      tasks: 0,
      subtasks: 0,
      points: 0,
      unestimatedLeaves: 0,
    })
  })
})
