import type { commands } from '$lib/ipc/bindings'
import type { UnsupportedCommand } from './unsupported'

type Commands = typeof commands

/** `getThreadEvents` to `get_thread_events`, the name the core registers it under. */
type SnakeCase<S extends string> = S extends `${infer Head}${infer Tail}`
  ? `${Head extends Uppercase<Head> ? `_${Lowercase<Head>}` : Head}${SnakeCase<Tail>}`
  : ''

type CommandsByWireName = {
  [K in keyof Commands & string as SnakeCase<K>]: Commands[K]
}

export type WireCommand = keyof CommandsByWireName

/** What the app gets once `commands.x()` unwraps: the `Result`'s data, or the plain value. */
type OkData<R> = R extends { status: 'ok'; data: infer T }
  ? T
  : R extends { status: 'error' }
    ? never
    : R

type Data<W extends WireCommand> = OkData<
  Awaited<ReturnType<CommandsByWireName[W]>>
>

/**
 * `args` is the object the bindings pass to invoke. TypeScript can't see the bindings' parameter
 * names, so each handler annotates the fields it reads; `never` makes that annotation mandatory.
 * The return type does come from the bindings.
 */
type Handler<W extends WireCommand> = (
  args: never,
) => Data<W> | Promise<Data<W>>

/** The arguments of the commands that take a single `input` struct. */
export type Input<W extends WireCommand> = {
  input: Parameters<CommandsByWireName[W]>[0]
}

/**
 * Every command needs an entry, unless `UNSUPPORTED_COMMANDS` lists it. A new command in
 * `bindings.ts` therefore fails `pnpm check` until it gets one or the other.
 */
export type HandlerTable = {
  [W in Exclude<WireCommand, UnsupportedCommand>]: Handler<W>
}

/** What one domain's file contributes to the table. */
export type HandlerSlice = Partial<HandlerTable>
