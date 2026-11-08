import ArrowRight from '@lucide/svelte/icons/arrow-right'
import Bot from '@lucide/svelte/icons/bot'
import Check from '@lucide/svelte/icons/check'
import Clock from '@lucide/svelte/icons/clock'
import FileText from '@lucide/svelte/icons/file-text'
import GitBranch from '@lucide/svelte/icons/git-branch'
import Globe from '@lucide/svelte/icons/globe'
import List from '@lucide/svelte/icons/list'
import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus'
import Pause from '@lucide/svelte/icons/pause'
import Pencil from '@lucide/svelte/icons/pencil'
import Play from '@lucide/svelte/icons/play'
import Search from '@lucide/svelte/icons/search'
import Sparkles from '@lucide/svelte/icons/sparkles'
import Square from '@lucide/svelte/icons/square'
import Terminal from '@lucide/svelte/icons/terminal'
import Trash2 from '@lucide/svelte/icons/trash-2'
import type { IconName } from './icon-name'

export const ICONS: Record<IconName, typeof Terminal> = {
  file: FileText,
  tool: Terminal,
  branch: GitBranch,
  check: Check,
  terminal: Terminal,
  database: Terminal,
  pr: GitBranch,
  download: ArrowRight,
  list: List,
  search: Search,
  pencil: Pencil,
  trash: Trash2,
  pause: Pause,
  play: Play,
  stop: Square,
  session: MessageSquarePlus,
  clock: Clock,
  globe: Globe,
  skill: Sparkles,
  agent: Bot,
}
