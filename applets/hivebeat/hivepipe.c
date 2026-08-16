/* hivepipe — stream s16le PCM from stdin to pulseaudio via the simple API.
 * replaces pacat (whose pa_stream_set_name call breaks against this daemon).
 * env: PULSE_SERVER=<socket>  HIVEPIPE_RATE  HIVEPIPE_CH */
#include <stdio.h>
#include <stdlib.h>
#include <pulse/simple.h>
#include <pulse/error.h>

int main(void) {
    const char *server = getenv("PULSE_SERVER");
    const char *rate_s = getenv("HIVEPIPE_RATE");
    const char *ch_s = getenv("HIVEPIPE_CH");
    pa_sample_spec ss;
    ss.format = PA_SAMPLE_S16LE;
    ss.rate = rate_s ? (unsigned)atoi(rate_s) : 48000u;
    ss.channels = ch_s ? (unsigned char)atoi(ch_s) : 2u;

    int err = 0;
    pa_simple *s = pa_simple_new(server, "hivebeat", PA_STREAM_PLAYBACK,
                                 NULL, "hivebeat", &ss, NULL, NULL, &err);
    if (!s) {
        fprintf(stderr, "hivepipe: pa_simple_new failed: %s\n", pa_strerror(err));
        return 1;
    }

    static unsigned char buf[8192];
    size_t n;
    while ((n = fread(buf, 1, sizeof(buf), stdin)) > 0) {
        if (pa_simple_write(s, buf, n, &err) < 0) {
            fprintf(stderr, "hivepipe: write failed: %s\n", pa_strerror(err));
            pa_simple_free(s);
            return 1;
        }
    }
    pa_simple_drain(s, &err);
    pa_simple_free(s);
    return 0;
}
