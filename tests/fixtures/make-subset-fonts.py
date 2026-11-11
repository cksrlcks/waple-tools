"""web.font_subset 테스트 폰트(subset-test.ttf / .otf / -var.ttf)를 만든다.

pip install fonttools 후 `python tests/fixtures/make-subset-fonts.py`
글자: ASCII 0x20-0x7E + 한글 몇 자(완성형 밖인 '똠' 포함) + 'Á' + U+1F600(보조 평면, cmap 포맷 12).
굵기 700, 기울임, GPOS 커닝(A-V). TTF의 'Á'는 A + 악센트(cmap에 없는 글리프) 합성 글리프다.
-var.ttf는 같은 TTF에 wght 100–900 축(fvar)을 더한 가변 폰트다.
글리프는 글자마다 모양이 다른 사각형이다.
"""

from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.pens.ttGlyphPen import TTGlyphPen

HANGUL = "가나다라마바사아자차카타파하한글웹폰트똠"
CHARS = [chr(c) for c in range(0x20, 0x7F)] + list(HANGUL) + ["Á", "\U0001F600"]
CMAP = {ord(c): f"uni{ord(c):04X}" if ord(c) <= 0xFFFF else f"u{ord(c):X}" for c in CHARS}
NAMES = [".notdef"] + list(CMAP.values()) + ["acute"]
FAMILY = "Waple Test"


def draw(pen, i):
    if i % 7 == 0:
        return  # 빈 글리프도 섞는다 (공백처럼)
    w = 100 + (i * 37) % 400
    pen.moveTo((50, 0))
    pen.lineTo((50, 700))
    pen.lineTo((50 + w, 700))
    pen.lineTo((50 + w, 0))
    pen.closePath()
    pen.moveTo((60 + i % 30, 10))
    pen.lineTo((40 + w, 10 + i % 50))
    pen.lineTo((40 + w, 690))
    pen.closePath()


def draw_acute(pen):
    pen.moveTo((200, 750))
    pen.lineTo((300, 900))
    pen.lineTo((350, 880))
    pen.closePath()


def draw_named(pen, n):
    if n == "acute":
        draw_acute(pen)
    elif n == "uni00C1" and not isinstance(pen, TTGlyphPen):
        # CFF에는 합성 글리프가 없어 A와 악센트를 한 윤곽으로 그린다
        draw(pen, NAMES.index("uni0041"))
        draw_acute(pen)
    elif n == "uni00C1":
        pen.addComponent("uni0041", (1, 0, 0, 1, 0, 0))
        pen.addComponent("acute", (1, 0, 0, 1, 0, 0))
    else:
        draw(pen, NAMES.index(n))


def build(cff: bool, out: Path, var: bool = False):
    fb = FontBuilder(1000, isTTF=not cff)
    fb.setupGlyphOrder(NAMES)
    fb.setupCharacterMap(CMAP)
    if cff:
        glyphs = {}
        for n in NAMES:
            pen = T2CharStringPen(600, None)
            draw_named(pen, n)
            glyphs[n] = pen.getCharString()
        fb.setupCFF(FAMILY.replace(" ", "") + "-BoldItalic", {"FullName": FAMILY}, glyphs, {})
    else:
        glyphs = {}
        for n in NAMES:
            pen = TTGlyphPen(dict.fromkeys(NAMES))  # 합성 글리프의 부품 이름 확인용
            draw_named(pen, n)
            glyphs[n] = pen.glyph()
        fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({n: (600, 50) for n in NAMES})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({
        "familyName": FAMILY,
        "styleName": "Bold Italic",
        "typographicFamily": FAMILY,
        "typographicSubfamily": "Bold Italic",
    })
    fb.setupOS2(usWeightClass=700, fsSelection=0x01 | 0x20, sTypoAscender=800, sTypoDescender=-200)
    fb.setupPost()
    fb.updateHead(macStyle=0b11)
    # GPOS 커닝 한 쌍 (A-V): 서브셋 엔진이 GPOS를 지키는지 본다
    fb.addOpenTypeFeatures("feature kern { pos uni0041 uni0056 -80; } kern;")
    if var:
        fb.setupFvar([("wght", 100, 700, 900, "Weight")], [])
        fb.setupGvar({})
    fb.save(out)


here = Path(__file__).parent
build(False, here / "subset-test.ttf")
build(True, here / "subset-test.otf")
build(False, here / "subset-test-var.ttf", var=True)
