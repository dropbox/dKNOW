def run0(self):
    """Run one item (a callback or an RPC wait_any).

    Returns:
      A time to sleep if something happened (may be 0);
      None if all queues are empty.
    """
    if self.current:
      self.inactive = 0
      callback, args, kwds = self.current.popleft()
      _logging_debug('nowevent: %s', callback.__name__)
      callback(*args, **kwds)
      return 0
    if self.run_idle():
      return 0
    delay = None
    if self.queue:
      delay = self.queue[0][0] - self.clock.now()
      if delay <= 0:
        self.inactive = 0
        _, callback, args, kwds = self.queue.pop(0)
        _logging_debug('event: %s', callback.__name__)
        callback(*args, **kwds)
        # TODO: What if it raises an exception?
        return 0
    if self.rpcs:
      self.inactive = 0
      rpc = datastore_rpc.MultiRpc.wait_any(self.rpcs)
      if rpc is not None:
        _logging_debug('rpc: %s.%s', rpc.service, rpc.method)
        # Yes, wait_any() may return None even for a non-empty argument.
        # But no, it won't ever return an RPC not in its argument.
        if rpc not in self.rpcs:
          raise RuntimeError('rpc %r was not given to wait_any as a choice %r' %
                             (rpc, self.rpcs))
        callback, args, kwds = self.rpcs[rpc]
        del self.rpcs[rpc]
        if callback is not None:
          callback(*args, **kwds)
          # TODO: Again, what about exceptions?
      return 0
    return delay