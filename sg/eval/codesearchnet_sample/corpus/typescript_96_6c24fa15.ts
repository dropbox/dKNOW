/** The new window scale factor. */
  scaleFactor: number
  /** The new window size */
  size: PhysicalSize
}

/**
 * Attention type to request on a window.
 *
 * @since 1.0.0
 */
enum UserAttentionType {
  /**
   * #### Platform-specific
   * - **macOS:** Bounces the dock icon until the application is in focus.
   * - **Windows:** Flashes both the window and the taskbar button until the application is in focus.
   */
  Critical = 1,
  /**
   * #### Platform-specific
   * - **macOS:** Bounces the dock icon once.
   * - **Windows:** Flashes the taskbar button until the application is in focus.
   */
  Informational
}