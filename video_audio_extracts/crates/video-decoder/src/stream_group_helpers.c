// Stream Group Helper Functions for FFmpeg 6.1+
// These functions provide safe access to stream group fields in AVFormatContext

#include <libavformat/avformat.h>

// Get number of stream groups in format context
unsigned int get_nb_stream_groups(AVFormatContext *fmt_ctx) {
    if (fmt_ctx == NULL) {
        return 0;
    }
    return fmt_ctx->nb_stream_groups;
}

// Get stream group at index
AVStreamGroup* get_stream_group(AVFormatContext *fmt_ctx, unsigned int index) {
    if (fmt_ctx == NULL || index >= fmt_ctx->nb_stream_groups) {
        return NULL;
    }
    return fmt_ctx->stream_groups[index];
}
