/**
 * Returns the path to the user's local data directory.
 *
 * #### Platform-specific
 *
 * - **Linux:** Resolves to `$XDG_DATA_HOME` or `$HOME/.local/share`.
 * - **macOS:** Resolves to `$HOME/Library/Application Support`.
 * - **Windows:** Resolves to `{FOLDERID_LocalAppData}`.
 * @example
 * ```typescript
 * import { localDataDir } from '@tauri-apps/api/path';
 * const localDataDirPath = await localDataDir();
 * ```
 *
 * @since 1.0.0
 */
async function localDataDir(): Promise<string> {
  return invoke('plugin:path|resolve_directory', {
    directory: BaseDirectory.LocalData
  })
}