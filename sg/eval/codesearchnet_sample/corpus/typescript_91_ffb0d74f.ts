/**
 * Emits an event to all {@link EventTarget|targets} matching the given target.
 *
 * @example
 * ```typescript
 * import { emitTo } from '@tauri-apps/api/event';
 * await emitTo('main', 'frontend-loaded', { loggedIn: true, token: 'authToken' });
 * ```
 *
 * @param target Label of the target Window/Webview/WebviewWindow or raw {@link EventTarget} object.
 * @param event Event name. Must include only alphanumeric characters, `-`, `/`, `:` and `_`.
 * @param payload Event payload.
 *
 * @since 2.0.0
 */
async function emitTo<T>(
  target: EventTarget | string,
  event: string,
  payload?: T
): Promise<void> {
  const eventTarget: EventTarget =
    typeof target === 'string' ? { kind: 'AnyLabel', label: target } : target
  await invoke('plugin:event|emit_to', {
    target: eventTarget,
    event,
    payload
  })
}