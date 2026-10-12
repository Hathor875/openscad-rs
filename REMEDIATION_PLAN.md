# Plan naprawczy dla openscad-rust

Na podstawie analizy 2 590 błędów z testu na całej bibliotece (3 709 plików), błędy nie rozkładają się losowo — ponad 60% awarii wynika z zaledwie kilku powtarzających się braków.

Poniższy plan podzielony jest na 5 faz wg kryterium ROI (stosunek zysku kompatybilności do nakładu pracy).

---

## 🎯 Cele i prognozowany wzrost kompatybilności

| Etap | Zakres prac | Szacowany czas | Oczekiwany wzrost | Łączna kompatybilność |
|---|---|:---:|:---:|:---:|
| **Faza 1** | Quick Wins: zmienne specjalne + wektory 2D | 1-2 dni | +450 plików (+12%) | **~42%** |
| **Faza 2** | Semantyka `undef` i tolerancyjna ewaluacja | 2-3 dni | +300 plików (+8%) | **~50%** |
| **Faza 3** | Rozszerzenie parsera (BOSL2 / list comprehensions) | 3-5 dni | +550 plików (+15%) | **~65%** |
| **Faza 4** | Brakujące built-iny i moduły 2D (`offset`, `projection`) | 3-5 dni | +350 plików (+9%) | **~74%** |
| **Faza 5** | Stabilność CSG, importy i testy regresyjne | Ciągły | +200 plików (+5%) | **~80%+** |

---

## 📋 Faza 1: Quick Wins (Błyskawiczne odblokowanie setek modeli)

- [ ] 1.1 Domyślne zmienne specjalne OpenSCAD (`$preview = false`, parametry widoku kamery `$vpr`, `$vpt`, `$vpd`, `$vpf`, `$children = 0`).
- [ ] 1.2 Niezdefiniowane identyfikatory ewaluowane jako `undef` zamiast błędu krytycznego.
- [ ] 1.3 Obsługa wektorów 2D w transformacjach 3D (`translate([x, y])` -> `[x, y, 0]`, `scale([x, y])` -> `[x, y, 1]`).
- [ ] 1.4 Konwersja wektorów na string w `Value::as_string()` i `str()`.

---

## 📋 Faza 2: Semantyka `undef` i tolerancyjna arytmetyka

- [ ] 2.1 Propagacja `undef` w operacjach binarnych i unarnych (np. `1 + undef -> undef`).
- [ ] 2.2 Bezpieczne indeksowanie i operacje na listach z `undef` poza zakresem.

---

## 📋 Faza 3: Parser i składnia BOSL2

- [ ] 3.1 Obsługa zagnieżdżonych `if / else if` wewnątrz generatorów pętli `[for (...) ...]`.
- [ ] 3.2 Opcjonalne średniki i literały w notacji naukowej.

---

## 📋 Faza 4: Brakujące moduły i funkcje wbudowane

- [ ] 4.1 Moduły geometrii 2D: `offset()` i `projection()`.
- [ ] 4.2 Funkcje wbudowane: `search()`, `chr()`, `ord()`, `version()`, `rands()`.

---

## 📋 Faza 5: Stabilność CSG, importy i testy regresyjne

- [ ] 5.1 Wyszukiwanie bibliotek w `~/.local/share/OpenSCAD/libraries`.
- [ ] 5.2 Automatyczny pipeline testowy weryfikujący brak regresji na 3709 plikach.
