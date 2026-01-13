/**
 * Function that should provides a logic for the response result
 * processing. Used as a part of a main configuration object of the
 * server-core to provide extendability for this logic.
 * @todo any could be changed to unknown?
 * @todo Maybe we can add type limitations?
 */
type ResponseResultFn =
  (
    message: (Record<string, any> | Record<string, any>[]) | DataResult | ErrorResponse,
    extra?: { status: number }
  ) => void | Promise<void>