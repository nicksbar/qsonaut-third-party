#include "fargan.h"
#include "lpcnet.h"
#include "cpu_support.h"

#include <stdlib.h>
#include <string.h>

#define RADE_SPEECH_FRAME_SAMPLES 160
#define RADE_SPEECH_FEATURES 36
#define RADE_FARGAN_WARMUP_FRAMES 5

typedef struct {
    LPCNetEncState *encoder;
    int arch;
} rade_speech_encoder;

typedef struct {
    FARGANState fargan;
    float warmup[RADE_FARGAN_WARMUP_FRAMES * RADE_SPEECH_FEATURES];
    int warmup_frames;
    int ready;
} rade_speech_decoder;

void *qsonaut_rade_speech_encoder_new(void) {
    rade_speech_encoder *encoder = calloc(1, sizeof(*encoder));
    if (!encoder) return NULL;
    encoder->encoder = lpcnet_encoder_create();
    encoder->arch = opus_select_arch();
    if (!encoder->encoder) {
        free(encoder);
        return NULL;
    }
    return encoder;
}

void qsonaut_rade_speech_encoder_free(void *opaque) {
    rade_speech_encoder *encoder = opaque;
    if (!encoder) return;
    lpcnet_encoder_destroy(encoder->encoder);
    free(encoder);
}

int qsonaut_rade_speech_encode_frame(
    void *opaque,
    const float *pcm,
    float *features
) {
    rade_speech_encoder *encoder = opaque;
    if (!encoder || !pcm || !features) return -1;

    /*
     * QSONaut's AudioBlock contract uses normalized float PCM, while the
     * upstream LPCNet feature extractor uses the signed-16-bit speech scale.
     * The upstream rade_tx_wav path performs this conversion before feature
     * extraction. Keep that conversion inside this bridge so every Rust
     * consumer gets the same LPCNet input contract.
     */
    float scaled_pcm[RADE_SPEECH_FRAME_SAMPLES];
    for (int index = 0; index < RADE_SPEECH_FRAME_SAMPLES; index++) {
        float sample = pcm[index] * 32768.0f;
        if (sample > 32767.0f) sample = 32767.0f;
        if (sample < -32767.0f) sample = -32767.0f;
        scaled_pcm[index] = sample;
    }
    return lpcnet_compute_single_frame_features_float(
        encoder->encoder, scaled_pcm, features, encoder->arch
    );
}

void *qsonaut_rade_speech_decoder_new(void) {
    rade_speech_decoder *decoder = calloc(1, sizeof(*decoder));
    if (!decoder) return NULL;
    fargan_init(&decoder->fargan);
    return decoder;
}

void qsonaut_rade_speech_decoder_free(void *opaque) {
    free(opaque);
}

int qsonaut_rade_speech_decode_frame(
    void *opaque,
    const float *features,
    float *pcm
) {
    rade_speech_decoder *decoder = opaque;
    if (!decoder || !features || !pcm) return -1;

    if (!decoder->ready) {
        memcpy(
            &decoder->warmup[decoder->warmup_frames * RADE_SPEECH_FEATURES],
            features,
            RADE_SPEECH_FEATURES * sizeof(float)
        );
        decoder->warmup_frames++;
        if (decoder->warmup_frames < RADE_FARGAN_WARMUP_FRAMES) return 0;

        float packed[RADE_FARGAN_WARMUP_FRAMES * 20];
        float zeros[320] = {0};
        for (int frame = 0; frame < RADE_FARGAN_WARMUP_FRAMES; frame++) {
            memcpy(
                &packed[frame * 20],
                &decoder->warmup[frame * RADE_SPEECH_FEATURES],
                20 * sizeof(float)
            );
        }
        fargan_cont(&decoder->fargan, zeros, packed);
        decoder->ready = 1;
        return 0;
    }

    fargan_synthesize(&decoder->fargan, pcm, features);
    return RADE_SPEECH_FRAME_SAMPLES;
}
