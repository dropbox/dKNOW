func (x *XRaySpanSink) Flush() {
	x.log.WithFields(logrus.Fields{
		"flushed_spans": atomic.LoadInt64(&x.spansHandled),
		"dropped_spans": atomic.LoadInt64(&x.spansDropped),
	}).Debug("Checkpointing flushed spans for X-Ray")
	metrics.ReportBatch(x.traceClient, []*ssf.SSFSample{
		ssf.Count(sinks.MetricKeyTotalSpansFlushed, float32(atomic.SwapInt64(&x.spansHandled, 0)), map[string]string{"sink": x.Name()}),
		ssf.Count(sinks.MetricKeyTotalSpansDropped, float32(atomic.SwapInt64(&x.spansDropped, 0)), map[string]string{"sink": x.Name()}),
	})
}