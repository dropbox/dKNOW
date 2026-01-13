function (query, callback) {
    if (!callback) {
      callback = query;
      query = null;
    }

    if (typeof query === 'string') {
      query = { aggregateId: query };
    }

    this.store.getUndispatchedEvents(query, callback);
  }