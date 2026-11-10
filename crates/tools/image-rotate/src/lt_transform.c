/* JPEG 무손실 변환 (mozjpeg jpegtran.c의 흐름을 메모리 입출력으로 줄인 것).
 * 반환: 0 성공, 1 손실 없이 변환 불가(MCU 경계가 안 맞음), 2 libjpeg 오류.
 * 성공하면 *out을 lt_free로 해제한다. */
#include <setjmp.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "jpeglib.h"
#include "transupp.h"

struct lt_state {
  struct jpeg_error_mgr err; /* 첫 필드여야 한다 (cinfo->err로 되돌려 받음) */
  jmp_buf jump;
  unsigned char *buf; /* setjmp 뒤에 바뀌는 값은 힙에 둔다 */
  unsigned long size;
};

static void lt_error_exit(j_common_ptr cinfo) {
  longjmp(((struct lt_state *)cinfo->err)->jump, 1);
}

static void lt_quiet(j_common_ptr cinfo, int level) {
  (void)cinfo;
  (void)level;
}

int lt_jpeg_transform(const unsigned char *in, unsigned long in_len, int op,
                      int copy_all_markers, unsigned char **out,
                      unsigned long *out_len) {
  static const JXFORM_CODE ops[] = {JXFORM_NONE,    JXFORM_FLIP_H,
                                    JXFORM_FLIP_V,  JXFORM_ROT_90,
                                    JXFORM_ROT_180, JXFORM_ROT_270};
  struct jpeg_decompress_struct src;
  struct jpeg_compress_struct dst;
  jpeg_transform_info xf;
  jvirt_barray_ptr *src_coef, *dst_coef;
  JCOPY_OPTION copy = copy_all_markers ? JCOPYOPT_ALL : JCOPYOPT_NONE;
  struct lt_state *st;

  if (op < 1 || op > 5) return 2;
  st = (struct lt_state *)calloc(1, sizeof(*st));
  if (!st) return 2;

  memset(&xf, 0, sizeof(xf));
  xf.transform = ops[op];
  xf.perfect = TRUE;

  src.err = jpeg_std_error(&st->err);
  st->err.error_exit = lt_error_exit;
  st->err.emit_message = lt_quiet;
  dst.err = &st->err;
  jpeg_create_decompress(&src);
  jpeg_create_compress(&dst);

  if (setjmp(st->jump)) {
    jpeg_destroy_compress(&dst);
    jpeg_destroy_decompress(&src);
    free(st->buf);
    free(st);
    return 2;
  }

  jpeg_mem_src(&src, in, in_len);
  jcopy_markers_setup(&src, copy);
  jpeg_read_header(&src, TRUE);
  if (!jtransform_request_workspace(&src, &xf)) {
    jpeg_destroy_compress(&dst);
    jpeg_destroy_decompress(&src);
    free(st);
    return 1;
  }
  src_coef = jpeg_read_coefficients(&src);
  jpeg_copy_critical_parameters(&src, &dst);
  dst_coef = jtransform_adjust_parameters(&src, &dst, src_coef, &xf);
  jpeg_mem_dest(&dst, &st->buf, &st->size);
  jpeg_write_coefficients(&dst, dst_coef);
  jcopy_markers_execute(&src, &dst, copy);
  jtransform_execute_transform(&src, &dst, src_coef, &xf);
  jpeg_finish_compress(&dst);
  jpeg_destroy_compress(&dst);
  (void)jpeg_finish_decompress(&src);
  jpeg_destroy_decompress(&src);

  *out = st->buf;
  *out_len = st->size;
  free(st);
  return 0;
}

void lt_free(unsigned char *p) { free(p); }
