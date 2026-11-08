// The composer's trigger words: `/skill`, `@repo`, `#task` and `!bang`. Typing one opens its
// picker; picking replaces the word.
import type { Bang, ClickupTask, RepoRecord, Skill } from '$lib/ipc/bindings'
import { taskRef } from '$lib/clickup/board'
import { filterBangs } from '$lib/bangs/bangs'
import { filterRepos, repoAlias } from './repo-aliases'
import { filterSkills } from './skill-picker'
import { filterTasks } from './task-aliases'

/** A trigger word in the draft: where it starts and ends, and what's typed before the caret. */
export type TriggerWord = { start: number; end: number; query: string }

/** Each trigger's sigil, and what its word is made of. */
const TRIGGERS = {
  // Not a path: `src/app` and `/usr/bin` aren't skills.
  skill: { sigil: '/', chars: '[^\\s/]' },
  // Not an email (`me@example.com`) or a scoped package (`@tauri-apps/api`).
  repo: { sigil: '@', chars: '[\\w.-]' },
  task: { sigil: '#', chars: '[\\w-]' },
  bang: { sigil: '!', chars: '[\\w-]' },
} as const

type TriggerKind = keyof typeof TRIGGERS

/** The `kind` word the caret is in, when it starts the draft or follows whitespace, or null. */
export function wordAt(
  kind: TriggerKind,
  draft: string,
  caret: number,
): TriggerWord | null {
  const { sigil, chars } = TRIGGERS[kind]
  const match = new RegExp(`(^|\\s)\\${sigil}(${chars}*)$`).exec(
    draft.slice(0, caret),
  )
  if (!match) return null
  const rest = /^\S*/.exec(draft.slice(caret))?.[0] ?? ''
  if (!new RegExp(`^${chars}*$`).test(rest)) return null
  return {
    start: match.index + (match[1]?.length ?? 0),
    end: caret + rest.length,
    query: match[2] ?? '',
  }
}

/**
 * The draft with `word` swapped for `text`, and where the caret goes. A picked `word` (`/name`)
 * is always followed by a space, with the caret after it; inserted `text` (a bang's prompt) is
 * only spaced from what follows it, with the caret right after the text.
 */
export function replaceWord(
  draft: string,
  word: TriggerWord,
  text: string,
  as: 'word' | 'text' = 'word',
): { text: string; caret: number } {
  const rest = draft.slice(word.end)
  const before = draft.slice(0, word.start)
  if (as === 'word') {
    const space = /^\s/.test(rest) ? '' : ' '
    return {
      text: before + text + space + rest,
      caret: word.start + text.length + 1,
    }
  }
  const gap = rest === '' || /^\s/.test(rest) ? '' : ' '
  return {
    text: before + text + gap + rest,
    caret: word.start + text.length + gap.length,
  }
}

/** What the pickers choose from; skills are null until they've been read. */
export type PickerSources = {
  skills: Skill[] | null
  repos: RepoRecord[]
  tasks: ClickupTask[]
  bangs: Bang[]
}

type Option = {
  key: string
  label: string
  /** What replaces the trigger word, and how (see `replaceWord`). */
  insert: string
  as: 'word' | 'text'
}

export type Picker =
  | { kind: 'skill'; word: TriggerWord; options: (Option & { item: Skill })[] }
  | {
      kind: 'repo'
      word: TriggerWord
      options: (Option & { item: RepoRecord })[]
    }
  | {
      kind: 'task'
      word: TriggerWord
      options: (Option & { item: ClickupTask })[]
    }
  | { kind: 'bang'; word: TriggerWord; options: (Option & { item: Bang })[] }

/**
 * The picker the caret's word opens, with what matches it, or null. Tasks and bangs only open
 * when there are some to offer.
 */
export function pickerAt(
  draft: string,
  caret: number,
  sources: PickerSources,
): Picker | null {
  const at = Math.min(caret, draft.length)
  const skill = wordAt('skill', draft, at)
  if (skill) {
    const skills = sources.skills
      ? filterSkills(sources.skills, skill.query)
      : []
    return {
      kind: 'skill',
      word: skill,
      options: skills.map((item) => ({
        item,
        key: item.name,
        label: `/${item.name}`,
        insert: `/${item.name}`,
        as: 'word',
      })),
    }
  }
  const repo = wordAt('repo', draft, at)
  if (repo) {
    return {
      kind: 'repo',
      word: repo,
      options: filterRepos(sources.repos, repo.query).map((item) => ({
        item,
        key: item.id,
        label: `@${repoAlias(item)}`,
        insert: `@${repoAlias(item)}`,
        as: 'word',
      })),
    }
  }
  const task = sources.tasks.length ? wordAt('task', draft, at) : null
  if (task) {
    return {
      kind: 'task',
      word: task,
      options: filterTasks(sources.tasks, task.query).map((item) => ({
        item,
        key: item.id,
        label: `#${taskRef(item)}`,
        insert: `#${taskRef(item)}`,
        as: 'word',
      })),
    }
  }
  const bang = sources.bangs.length ? wordAt('bang', draft, at) : null
  if (bang) {
    return {
      kind: 'bang',
      word: bang,
      options: filterBangs(sources.bangs, bang.query).map((item) => ({
        item,
        key: item.id,
        label: `!${item.name}`,
        insert: item.prompt.trim(),
        as: 'text',
      })),
    }
  }
  return null
}
