# Plan Projektu: Morfologik-rs - Przepisanie z Javy na Rusta

## 1. Wprowadzenie i Cele

Celem projektu jest przepisanie istniejącej biblioteki Java `morfologik-stemming` na język Rust. Głównymi motywacjami są potencjalne korzyści w zakresie wydajności, bezpieczeństwa pamięci oraz możliwość łatwiejszej integracji z innymi systemami napisanymi w Rust. Projekt `morfologik-rs` ma na celu dostarczenie funkcjonalnie równoważnej biblioteki z zachowaniem (lub poprawą) wydajności oryginału.

Projekt będzie obejmował następujące główne komponenty oryginalnej biblioteki:
* Obsługę automatów skończonych (FSA) - odczyt, zapis, operacje.
* Logikę stemmingu i obsługę słowników.
* Implementację specyficznego stemera dla języka polskiego.
* Narzędzia linii komend do pracy ze słownikami i automatami FSA.
* Opcjonalnie: logikę spellera.

## 2. Analiza Istniejącej Bazy Kodu Java

Przed przystąpieniem do implementacji w Rust, konieczna jest dogłębna analiza kodu Java.

* **Struktura Projektu:**
    * Zidentyfikowanie głównych modułów Maven:
        * `morfologik-fsa-builders`: Narzędzia do budowania automatów FSA.
        * `morfologik-fsa`: Rdzenna biblioteka do obsługi FSA (odczyt, traversacja).
        * `morfologik-stemming`: Główna logika stemmingu, definicje słowników, interfejsy stemerów.
        * `morfologik-polish`: Implementacja stemera dla języka polskiego.
        * `morfologik-speller`: Funkcjonalność sprawdzania pisowni.
        * `morfologik-tools`: Narzędzia linii komend.
    * Zrozumienie zależności między modułami.
* **Kluczowe Klasy i Interfejsy:**
    * **FSA:** `FSA.java`, `FSA5.java`, `CFSA.java`, `CFSA2.java`, `FSABuilder.java`, `FSASerializer.java` (oraz jego implementacje np. `CFSA2Serializer.java`, `FSA5Serializer.java`).
    * **Stemming:** `Dictionary.java`, `DictionaryLookup.java`, `IStemmer.java`, `WordData.java`, `EncoderType.java`, `ISequenceEncoder.java` (i implementacje).
    * **Polish Stemmer:** `PolishStemmer.java`.
    * **Speller:** `Speller.java`, `HMatrix.java`.
    * **Tools:** Klasy w pakiecie `morfologik.tools` (np. `DictCompile.java`, `FSABuild.java`).
* **Formaty Danych:**
    * Szczegółowe zrozumienie formatów binarnych automatów FSA (FSA5, CFSA2).
    * Format plików słownikowych (`.dict`) i metadanych (`.info`).
    * Sposób kodowania sekwencji (np. tagów, form podstawowych).
* **Algorytmy:**
    * Algorytmy budowania FSA.
    * Algorytmy traversacji i wyszukiwania w FSA.
    * Logika działania stemerów i enkoderów.
    * Algorytmy używane w spellerze.
* **Testy:**
    * Przegląd istniejących testów jednostkowych i integracyjnych w Javie. Mogą one posłużyć jako podstawa do tworzenia testów w Rust i weryfikacji poprawności implementacji.

## 3. Planowanie Implementacji w Rust

* **Struktura Projektu Rust:**
    * Zalecane jest użycie Rust Workspace.
    * Każdy główny moduł Javy (np. `morfologik-fsa`, `morfologik-stemming`) powinien mieć swój odpowiednik jako `crate` w workspace `morfologik-rs`.
        * `morfologik-fsa-rs`: Obsługa FSA (odczyt, struktury danych).
        * `morfologik-fsa-builders-rs`: Budowanie FSA.
        * `morfologik-stemming-rs`: Rdzeń logiki stemmingu, definicje słowników.
        * `morfologik-polish-rs`: Stemer dla języka polskiego.
        * `morfologik-speller-rs` (opcjonalnie): Speller.
        * `morfologik-tools-rs`: Narzędzia CLI.
* **Wybór Bibliotek (Crates):**
    * Do obsługi plików i I/O: standardowa biblioteka Rusta (`std::fs`, `std::io`).
    * Do parsowania argumentów linii komend: `clap`.
    * Do serializacji/deserializacji (jeśli potrzebne dla wewnętrznych struktur, poza formatami FSA): `serde`.
    * Do obsługi map bitowych/efektywnego zarządzania pamięcią: rozważyć `bitvec` lub podobne.
    * Do testowania: wbudowane mechanizmy Rusta, ewentualnie `proptest` dla testów właściwości.
* **Projektowanie API:**
    * API powinno być idiomatyczne dla Rusta (wykorzystanie `Result`, `Option`, cech (traits), zarządzanie własnością).
    * Należy dążyć do zachowania podobnej funkcjonalności jak w API Javy, aby ułatwić migrację użytkownikom, ale nie kosztem jakości kodu Rust.
    * Zdefiniowanie publicznych interfejsów (traits) dla kluczowych komponentów (np. `FsaReader`, `Stemmer`).
* **Zarządzanie Błędami:**
    * Spójny system obsługi błędów przy użyciu `Result` i dedykowanych typów błędów dla każdego `crate`.
* **Konwencje Kodowania:**
    * Stosowanie `rustfmt` do formatowania kodu.
    * Stosowanie `clippy` do analizy statycznej i sugestii.

## 4. Fazy Implementacji

Implementacja będzie podzielona na fazy, zaczynając od fundamentalnych komponentów.

### Faza 1: Rdzeń Obsługi FSA (`morfologik-fsa-rs`, `morfologik-fsa-builders-rs`)

* **Cel:** Implementacja odczytu, zapisu i podstawowych operacji na automatach FSA.
* **Zadania:**
    1.  Zdefiniowanie struktur danych w Rust do reprezentacji automatów FSA (wersje FSA5, CFSA2).
    2.  Implementacja logiki odczytu nagłówków FSA.
    3.  Implementacja deserializacji automatów FSA z formatów binarnych (FSA5, CFSA2) do struktur Rust.
        * Kluczowe klasy Java: `FSA.java`, `FSA5.java`, `CFSA2.java`, `FSAHeader.java`.
    4.  Implementacja funkcji do traversacji FSA (np. `getRootNode()`, `getNextArc()`, `getArc()` `isArcFinal()`, `isArcTerminal()`).
        * Kluczowe klasy Java: `FSATraversal.java`.
    5.  Implementacja funkcji wyszukiwania sekwencji w FSA.
    6.  Implementacja logiki budowania automatów FSA (`FSABuilder.java`).
        * Zrozumienie algorytmów minimalizacji i optymalizacji.
    7.  Implementacja serializacji automatów FSA do formatów binarnych (FSA5, CFSA2).
        * Kluczowe klasy Java: `FSASerializer.java`, `CFSA2Serializer.java`, `FSA5Serializer.java`.
* **Testowanie:**
    * Testy jednostkowe dla odczytu/zapisu znanych plików FSA.
    * Testy traversacji i wyszukiwania.
    * Porównanie zbudowanych automatów z tymi z Javy (jeśli możliwe, np. przez dekompilację).

### Faza 2: Logika Stemmingu (`morfologik-stemming-rs`)

* **Cel:** Implementacja ogólnej logiki stemmingu, obsługi słowników i enkoderów.
* **Zadania:**
    1.  Zdefiniowanie struktur danych dla słownika (`Dictionary`), danych słowa (`WordData`), metadanych słownika (`DictionaryMetadata`).
        * Kluczowe klasy Java: `Dictionary.java`, `WordData.java`, `DictionaryMetadata.java`, `DictionaryMetadataBuilder.java`.
    2.  Implementacja odczytu plików `.dict` i `.info`. Plik `.dict` zawiera FSA.
    3.  Zdefiniowanie cechy (trait) `Stemmer` (odpowiednik `IStemmer`).
    4.  Implementacja `DictionaryLookup` do wyszukiwania form słów i ich tagów/form podstawowych.
        * Kluczowe klasy Java: `DictionaryLookup.java`.
    5.  Implementacja różnych enkoderów sekwencji (odpowiedniki `ISequenceEncoder` i jego implementacji: `NoEncoder`, `TrimSuffixEncoder`, `TrimPrefixAndSuffixEncoder`, `TrimInfixAndSuffixEncoder`).
        * Zrozumienie, jak separatory i operacje na sekwencjach (np. odcinanie prefixu/suffixu) są realizowane.
* **Testowanie:**
    * Testy jednostkowe ładowania słowników.
    * Testy wyszukiwania słów i odtwarzania ich form.
    * Testy działania enkoderów na przykładowych danych.

### Faza 3: Stemer dla Języka Polskiego (`morfologik-polish-rs`)

* **Cel:** Implementacja konkretnego stemera dla języka polskiego.
* **Zadania:**
    1.  Implementacja struktury `PolishStemmer` implementującej cechę `Stemmer`.
    2.  Wykorzystanie `morfologik-stemming-rs` do ładowania odpowiedniego słownika polskiego.
    3.  Adaptacja logiki specyficznej dla języka polskiego, jeśli taka istnieje poza standardowym mechanizmem słownikowym.
        * Kluczowa klasa Java: `PolishStemmer.java`.
* **Testowanie:**
    * Testy jednostkowe dla stemera polskiego, wykorzystujące te same przypadki testowe co w `PolishMorfologikStemmerTest.java`.
    * Porównanie wyników z oryginalnym stemerem Java.

### Faza 4: Narzędzia Linii Komend (`morfologik-tools-rs`)

* **Cel:** Przepisanie narzędzi CLI do Rusta.
* **Zadania:**
    1.  Dla każdego narzędzia Javy (np. `FSABuild`, `FSACompile`, `FSADump`, `FSAInfo`, `DictCompile`, `DictDecompile`, `DictApply`):
        * Zdefiniowanie argumentów linii komend przy użyciu `clap`.
        * Implementacja logiki narzędzia, wykorzystując stworzone wcześniej `crate`y (`morfologik-fsa-rs`, `morfologik-stemming-rs`, etc.).
        * Obsługa wejścia/wyjścia (pliki, stdin/stdout).
        * Zarządzanie statusami wyjścia.
* **Testowanie:**
    * Testy integracyjne dla każdego narzędzia CLI, porównujące wyniki z narzędziami Javy.
    * Testowanie obsługi błędów i różnych opcji linii komend.

### Faza 5: Speller (`morfologik-speller-rs`) - Opcjonalnie

* **Cel:** Implementacja funkcjonalności sprawdzania pisowni.
* **Zadania:**
    1.  Analiza implementacji `Speller.java` i `HMatrix.java`.
    2.  Zdefiniowanie odpowiednich struktur i algorytmów w Rust.
    3.  Integracja ze słownikami.
* **Testowanie:**
    * Testy jednostkowe dla logiki spellera.

## 5. Testowanie i Walidacja

Testowanie jest kluczowym elementem projektu, aby zapewnić poprawność i niezawodność implementacji Rust.

* **Testy Jednostkowe:** Każdy `crate` i moduł powinien mieć rozbudowany zestaw testów jednostkowych.
* **Testy Integracyjne:** Testowanie współpracy między `crate`ami.
* **Testy Porównawcze (Golden Tests):**
    * Użycie istniejących plików testowych (słowników, automatów FSA, danych wejściowych dla stemerów/narzędzi) z projektu Java.
    * Uruchomienie zarówno implementacji Java, jak i Rust na tych samych danych i porównanie wyników. Wszelkie rozbieżności muszą być dokładnie zbadane.
* **Testy Wydajnościowe (Benchmarking):**
    * Przygotowanie zestawu benchmarków dla kluczowych operacji (np. ładowanie słownika, stemming dużej liczby słów, budowanie FSA).
    * Porównanie wydajności implementacji Rust z Javą. Celem jest osiągnięcie co najmniej porównywalnej, a najlepiej lepszej wydajności. Wykorzystanie `cargo bench`.
* **Fuzzing:** Rozważenie użycia fuzzingu (np. z `cargo-fuzz`) do testowania parserów formatów binarnych (FSA) w celu wykrycia potencjalnych luk bezpieczeństwa lub błędów obsługi niepoprawnych danych.

## 6. Dokumentacja i Publikacja

* **Dokumentacja Kodu:**
    * Generowanie dokumentacji API przy użyciu `rustdoc`.
    * Komentarze dokumentacyjne dla wszystkich publicznych funkcji, struktur, cech i modułów.
* **Przykłady Użycia:**
    * Stworzenie przykładów użycia biblioteki jako `crate` oraz narzędzi CLI.
* **README:**
    * Szczegółowy plik README dla głównego `workspace` oraz dla poszczególnych `crate`ów, wyjaśniający jak budować, testować i używać biblioteki.
* **Publikacja:**
    * Przygotowanie i opublikowanie `crate`ów na `crates.io`.
    * Zarządzanie wersjonowaniem.

## 7. Zarządzanie Projektem i Współpraca

* **System Kontroli Wersji:** Użycie Git, repozytorium na platformie takiej jak GitHub lub GitLab.
* **Śledzenie Zadań (Issue Tracking):** Użycie wbudowanego systemu śledzenia zadań (np. GitHub Issues) do zarządzania zadaniami, błędami i propozycjami ulepszeń.
* **Przeglądy Kodu (Code Reviews):** Regularne przeglądy kodu są kluczowe dla utrzymania wysokiej jakości kodu i dzielenia się wiedzą.
* **Ciągła Integracja (CI):** Skonfigurowanie CI (np. GitHub Actions) do automatycznego budowania, testowania i uruchamiania `clippy` oraz `rustfmt` przy każdym pushu/pull requeście.

## 8. Potencjalne Wyzwania i Ryzyka

* **Złożoność Formatów Binarnych FSA:** Dokładne odtworzenie logiki czytania i pisania formatów FSA może być trudne i czasochłonne.
* **Subtelne Różnice w Zachowaniu:** Mogą wystąpić drobne różnice w zachowaniu między implementacją Java a Rust, które będą trudne do wykrycia.
* **Wydajność:** Osiągnięcie oczekiwanej wydajności w Rust może wymagać starannej optymalizacji.
* **Kompletność Testów:** Zapewnienie, że zestaw testów jest wystarczająco kompletny, aby wychwycić wszystkie istotne błędy.
* **Zakres Projektu:** Przepisanie całej biblioteki, włącznie ze wszystkimi narzędziami i spellerem, jest dużym przedsięwzięciem. Może być konieczne ustalenie priorytetów i ewentualne podzielenie projektu na mniejsze, zarządzalne części.

## 9. Harmonogram (Wstępny)

Harmonogram jest bardzo orientacyjny i zależy od dostępnych zasobów.

* **Miesiąc 1-2:** Analiza kodu Java, planowanie implementacji Rust, konfiguracja projektu i środowiska.
* **Miesiąc 2-4:** Implementacja Fazy 1 (Rdzeń FSA).
* **Miesiąc 4-6:** Implementacja Fazy 2 (Logika Stemmingu).
* **Miesiąc 6-7:** Implementacja Fazy 3 (Stemer dla Języka Polskiego).
* **Miesiąc 7-9:** Implementacja Fazy 4 (Narzędzia Linii Komend).
* **Miesiąc 9-10 (Opcjonalnie):** Implementacja Fazy 5 (Speller).
* **Ciągle:** Testowanie, dokumentacja, refaktoryzacja.

Ten plan stanowi ramy dla projektu `morfologik-rs`. Poszczególne etapy i zadania mogą wymagać dalszego uszczegółowienia w miarę postępu prac.
