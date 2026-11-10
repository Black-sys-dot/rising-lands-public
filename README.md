# Rising Lands — motor reimplementado (macOS)

[English version](README.en.md)

Este repositório contém uma reimplementação do motor do jogo **Rising Lands** (1997) em Rust e um aplicativo para macOS que roda o jogo sobre esse motor. A lógica foi portada rotina por rotina do executável original e comparada com ele.

**O jogo não vem junto.** Para jogar, você precisa da imagem do **CD original brasileiro** do Rising Lands. O aplicativo lê todos os dados do jogo (gráficos, sons, música, textos, fases e filmes) desse CD na primeira execução. Nenhum dado do jogo é distribuído aqui.

## Estado atual

- O motor e o aplicativo compilam a partir deste repositório (veja abaixo).
- **Ainda falta uma etapa para jogar com um build feito daqui:** algumas tabelas que o renderizador usa (paleta, tabelas de sombra e de iluminação, cache inicial de terreno e as imagens das telas do menu) ainda são preparadas fora do aplicativo. Elas passarão a ser geradas a partir do CD do jogador na primeira execução. Enquanto isso não estiver pronto, o aplicativo compilado daqui abre, mas avisa que o manifesto de importação está ausente. Os detalhes estão em [PUBLICATION_AUDIT.md](PUBLICATION_AUDIT.md).

## Requisitos

- Mac com **Apple Silicon** (arm64) e **macOS 13** ou mais recente.
- **Xcode Command Line Tools**: `xcode-select --install`.
- **Rust** estável, instalado com o rustup (https://rustup.rs).
- A imagem do CD original brasileiro (veja "Onde conseguir o CD").

## Compilar

```bash
./build-macos.sh
```

O aplicativo fica em `build/RisingLands.app`. Para escolher outra pasta de saída: `./build-macos.sh <pasta>`.

Testes:

```bash
cargo test --locked --manifest-path engine/Cargo.toml
```
```bash
./macos/tests/run.sh
```

Alguns testes do motor precisam dos arquivos originais do jogo e ficam ignorados quando eles não estão presentes.

## Onde conseguir o CD

A imagem do CD original brasileiro está disponível no **Internet Archive**. Baixe os arquivos **RISING_LANDS_BR_.BIN** e **RISING_LANDS_BR_.CUE**. A música do jogo só vem da imagem **.BIN**.

## Rodar e importar o CD

1. Abra `build/RisingLands.app`. Se o macOS bloquear por ser um aplicativo não notarizado, clique com o botão direito no aplicativo e escolha **Abrir**.
2. Na primeira execução, a janela **"Rising Lands — importar o CD original"** pede o CD. Clique em **Escolher o CD…** e selecione o arquivo **RISING_LANDS_BR_.BIN** (ou o .CUE, um .ISO ou a pasta com os arquivos do CD).
3. Os arquivos são conferidos e copiados para a pasta de Suporte de Aplicativos uma única vez. Depois disso, o jogo abre direto. Para importar de novo: **Jogo → Importar o CD novamente…**.

## Controles

- **Velocidade:** menu **Jogo → Velocidade**, com 0,5x, 1x, 2x e 4x (15, 30, 60 e 120 quadros por segundo). A velocidade normal (1x) é 30 quadros por segundo. Atalhos: **⌘]** mais rápido, **⌘[** mais devagar. **⌘P** pausa e continua.
- **Tela cheia:** menu **Janela → Tela cheia**. Em tela cheia aparece só o jogo, sobre fundo preto.
- **Andar pelo mapa:** encoste o mouse na borda. Em tela cheia, basta levar o mouse até a borda da tela. Em janela, o mapa rola enquanto o mouse estiver dentro da janela, junto à borda. As setas do teclado também rolam o mapa.
- **Teclado:** Enter, Esc, Espaço, setas, Delete, X, Tab (minimapa) e os números 0–9, como no original. As teclas podem ser trocadas em **Jogo → Configurar teclas…**. A tecla F11 do original (trocar para 640x480) não é usada.

## Problemas

Abra uma *issue* neste repositório com o modelo **Problema no jogo**. Ajuda muito informar:
- a versão do macOS e o modelo do Mac;
- a fase e o que você estava fazendo;
- o texto inteiro do painel de parada, se ele apareceu (ele traz o endereço da rotina do original).

Não anexe saves nem arquivos do jogo: eles podem conter dados do jogo original.

## Licença

O código deste repositório é distribuído sob a licença MIT (veja `LICENSE`): qualquer pessoa pode ler, usar, modificar e redistribuir. A licença cobre só o código; não cobre o jogo original.

## Direitos

Rising Lands e todos os seus materiais originais (gráficos, sons, música, textos, fases, filmes e executável) continuam pertencendo aos seus titulares. Este repositório não distribui nenhum desses materiais e não concede licença sobre eles. O jogador usa a sua própria cópia do CD.

Licença do código deste repositório: **a definir** (TODO).
