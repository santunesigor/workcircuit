# WorkCircuit

[English](README.md) · [Português (Brasil)](README.pt-BR.md)

Um aplicativo desktop para Linux que funciona offline, ajuda você a alternar entre projetos e mostra onde seu tempo foi gasto.

Enquanto uma IA trabalha em um projeto, você pode seguir para outro sem perder sua posição no fluxo. O WorkCircuit mantém uma rotação de tarefas, registra o tempo trabalhado e oferece uma minijanela que fica acima das outras janelas.

**Última versão publicada: 0.1.0** · [Releases](https://github.com/santunesigor/workcircuit/releases) · [Histórico de alterações](CHANGELOG.md)

A conclusão de tarefas e as tarefas temporárias estão disponíveis na compilação do código atual; ainda não estão incluídas no pacote da release 0.1.0. Sincronização na nuvem e distribuição por APT/Snap estão planejadas.

## O que ele faz

- Cria tarefas com título obrigatório e descrição opcional; permite editar, reordenar, arquivar e restaurar.
- Mantém uma fila circular, com opção de avançar ou selecionar outra tarefa diretamente.
- Permite concluir tarefas e reabri-las sem perder o histórico.
- Inicia tarefas temporárias usando apenas um nome, para reuniões ou outras interrupções. Elas são concluídas automaticamente quando você passa para outra tarefa.
- Conta tempo em uma única tarefa por vez, com pausa e retomada manual.
- Respeita intervalos recorrentes por dia da semana e horário local, inclusive durante a madrugada.
- Oferece lembretes opcionais para verificar a próxima tarefa.
- Mostra relatórios por período e exporta totais e sessões em CSV.
- Salva tudo no computador, sem conta, internet ou servidor de banco de dados.
- Vem em inglês por padrão e permite salvar a preferência por português brasileiro.

## Instalar uma release

Baixe o arquivo Linux x86_64 e o `SHA256SUMS` na [última release](https://github.com/santunesigor/workcircuit/releases/latest). O binário é compilado no Ubuntu 24.04 e precisa de GTK 4.12+, libadwaita 1.4+ e X11/XWayland. Outras distribuições com bibliotecas compatíveis podem funcionar, mas ainda não são alvos de distribuição.

```sh
sudo apt install libgtk-4-1 libadwaita-1-0 xwayland
sha256sum --check SHA256SUMS
tar -xzf workcircuit-0.1.0-linux-x86_64.tar.gz
cd workcircuit-0.1.0-linux-x86_64
./install.sh
```

O instalador usa `~/.local` por padrão. Procure **WorkCircuit** no menu de aplicativos do GNOME e, se quiser, fixe o ícone no dock com **Adicionar aos favoritos**.

Para desinstalar, remova `~/.local/bin/workcircuit`, `~/.local/share/applications/io.github.workcircuit.WorkCircuit.desktop` e os dois arquivos `io.github.workcircuit.WorkCircuit*.svg` em `~/.local/share/icons/hicolor/scalable/apps/`. O banco de dados é preservado.

## Compilar o código atual

Requisitos: Linux, Rust **1.92+**, GTK **4.12+** com suporte a X11, libadwaita **1.4+** e ferramentas de compilação. No Wayland, a minijanela precisa do XWayland. Ubuntu 24.04 ou mais recente atende aos requisitos básicos.

Em Ubuntu/Debian com versões compatíveis de GTK:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev xwayland
git clone https://github.com/santunesigor/workcircuit.git
cd workcircuit
cargo build --locked
./target/debug/workcircuit
```

Para compilar e instalar a versão otimizada em `~/.local`:

```sh
make install
```

Ou apenas compilar e executar:

```sh
cargo build --release --locked
./target/release/workcircuit
```

O primeiro download das dependências precisa de internet. O aplicativo compilado funciona offline. SQLite é incluído no aplicativo; não é necessário instalar PostgreSQL nem um servidor SQLite. Para iniciar pelo terminal usando `workcircuit`, inclua `~/.local/bin` no `PATH`. O lançador do GNOME usa o caminho absoluto do executável.

## Como usar

1. Adicione seus projetos e organize a sequência com os botões de subir/descer.
2. Selecione uma tarefa e pressione **Start / Iniciar**. Selecionar uma tarefa enquanto o cronômetro está pausado não inicia a contagem.
3. Use **Next / Próxima** para encerrar a sessão atual e seguir para a próxima no mesmo instante. A fila volta ao início depois da última tarefa. Se houver apenas uma tarefa comum, ela continua rodando.
4. Se precisar pular tarefas, selecione outra diretamente. As tarefas puladas não recebem tempo e a rotação continua a partir da selecionada.
5. Use **Pause / Pausar** para um intervalo e **Resume / Retomar** ao voltar.
6. Abra **Reports / Relatórios**, informe um período de datas locais e consulte ou exporte o resultado. Pressione **Show / Exibir** novamente para atualizar o tempo da sessão em andamento.

O cronômetro mostra o tempo acumulado na passagem atual pela tarefa, descontando pausas. Trocar de tarefa inicia uma nova passagem; os relatórios preservam todas as sessões anteriores.

### Concluir e reabrir tarefas

Use **Complete / Concluir** no painel ou o botão **✓** ao lado da tarefa ou no cabeçalho da minijanela. A tarefa sai da rotação e aparece em **Completed tasks / Tarefas concluídas**, mantendo seu tempo nos relatórios e nas exportações.

Ao concluir a tarefa atual com o cronômetro rodando, a próxima começa no mesmo instante. Se estiver pausado, apenas a seleção muda e a contagem continua pausada. Concluir a última tarefa encerra a contagem.

**Reopen / Reabrir** devolve a tarefa ao final da fila com a mesma identidade e histórico. Arquivar é uma ação separada, que também preserva o histórico. Arquivar a tarefa atual pausa a contagem; **Restore / Restaurar** devolve uma tarefa arquivada ao final da fila.

### Tarefas temporárias para reuniões e interrupções

Pressione **Temporary Task / Tarefa temporária** no painel ou **+** na minijanela. Digite apenas o nome e pressione **Start Now / Iniciar agora**, ou Enter. Ela começa imediatamente, inclusive se você estava pausado. Se outra tarefa estava rodando, a sessão anterior termina exatamente no instante em que a temporária começa. Fechar o formulário sem iniciar mantém seu trabalho como estava.

A temporária entra imediatamente antes da tarefa interrompida. Por exemplo, se você estava em B na fila A, B, C, iniciar “Reunião” interrompe B; ao pressionar **Próxima**, “Reunião” é concluída e B volta a rodar. A sequência normal continua sendo A, B, C.

Selecionar outra tarefa diretamente também conclui a temporária. Se ela for a única tarefa ativa, **Próxima** a conclui e encerra a contagem. Iniciar uma segunda temporária conclui a primeira.

Pausar, entrar em um intervalo agendado, suspender o computador ou reiniciar o aplicativo não conclui a tarefa temporária. Retome quando quiser e avance ou troque de tarefa para finalizá-la. Durante um intervalo agendado, a criação de temporárias fica bloqueada.

O tempo da reunião permanece em **Tarefas concluídas**, nos relatórios e nos CSVs. Reabrir uma temporária mantém esse comportamento de conclusão automática.

### Minijanela no GNOME

Pressione **Overlay / Minijanela** para abrir o painel compacto de aproximadamente 300 × 140 pixels. Ele solicita automaticamente a posição acima das janelas comuns. Arraste a área do nome para movê-lo, use a lista para selecionar tarefas e o botão de expandir para voltar ao painel principal.

No GNOME/Wayland, apenas a minijanela usa XWayland; o painel principal continua nativo. Aplicativos em tela cheia e superfícies do sistema seguem as regras do compositor. Se XWayland não estiver disponível, o app mostra uma orientação para instalá-lo ou ativá-lo.

Fechar uma janela mantém a outra funcionando. Fechar a última janela encerra a contagem. O ícone de duas setas acompanha as cores de primeiro plano dos temas claro e escuro.

### Idioma

Abra **Settings → Language**, selecione **Português (Brasil)** e pressione **Save Settings**. As duas janelas mudam de idioma sem interromper a sessão atual. A preferência é salva; novas instalações começam em inglês.

Títulos e descrições continuam como foram digitados. Os nomes das colunas CSV permanecem em inglês.

### Intervalos e lembretes

Em **Configurações**, adicione intervalos por dia da semana, com horários locais no formato de 24 horas. Intervalos sobrepostos são tratados como um único período. Se atravessar a meia-noite, o dia escolhido é o dia de início. Início e fim iguais são rejeitados.

O aplicativo impede iniciar ou retomar durante o intervalo; depois que ele termina, você retoma manualmente. O lembrete é desativado por padrão. Configure os minutos para receber um aviso por passagem na tarefa. Pausas preservam o tempo acumulado e o aviso nunca troca de tarefa sozinho. Notificações dependem das configurações do desktop; o app também mostra uma mensagem.

## Dados, relatórios e backup

O banco fica em:

```text
$XDG_DATA_HOME/workcircuit/workcircuit.db
# padrão: ~/.local/share/workcircuit/workcircuit.db
```

Tarefas, sessões e regras de intervalo têm UUIDs estáveis. Mudanças de esquema usam migrações transacionais numeradas, aplicadas automaticamente. Transições de tarefas são salvas junto com as sessões; uma sessão ativa é salva a cada cinco segundos. Após uma falha, o app encerra a sessão no último registro salvo e reabre pausado. Até cinco segundos ainda não salvos podem ser perdidos.

Os horários vêm do computador, são armazenados em UTC e exibidos no fuso local. A duração usa um relógio monotônico. A suspensão pausa a contagem via logind, com detecção de lacunas como alternativa; interrupções longas são tratadas conservadoramente para evitar contar tempo ausente.

Os relatórios consideram apenas a parte de cada sessão dentro do período escolhido. Se o relógio mudar durante a sessão, a duração medida é distribuída proporcionalmente pela sobreposição dos horários. Intervalos de relógio não positivos são atribuídos à data de início. Os CSVs usam horários UTC, duração em segundos, escape de texto e proteção contra fórmulas de planilha em células textuais.

Para fazer backup, feche o WorkCircuit e copie o arquivo do banco. Não compartilhe um SQLite aberto entre computadores por pasta sincronizada ou unidade de rede. CSV é um relatório, não um backup completo do aplicativo.

## Arquitetura e evolução

Um único crate Rust separa as responsabilidades:

| Módulo | Responsabilidade |
| --- | --- |
| `domain` | Rotação, sessões, conclusão, temporárias, intervalos e lembretes |
| `storage` | SQLite, migrações, gravação atômica e recuperação |
| `report` | Recorte por período, agregação e CSV |
| `ui` | Janelas GTK/libadwaita, notificações e suspensão |
| `i18n` | Textos em inglês e português brasileiro |
| `overlay` | Display X11 dedicado e solicitação de janela acima das demais |

O domínio não depende de GTK ou SQL. As duas janelas compartilham o mesmo controlador. Consultas por período usam índices e não há gravações por segundo no banco.

A direção futura é manter SQLite local em cada PC e adicionar uma API autenticada de sincronização, possivelmente com PostgreSQL no servidor. Os IDs estáveis e a separação de módulos ajudam essa evolução, mas sincronização ainda não está implementada. Identidade de dispositivos, exclusões, sessões simultâneas e resolução de conflitos precisam ser especificadas antes do serviço.

## Desenvolvimento e especificações

Os requisitos atuais estão em [`openspec/specs`](openspec/specs); a proposta inicial e seu histórico estão em [`openspec/changes/archive`](openspec/changes/archive). OpenSpec é uma ferramenta de desenvolvimento, não uma dependência para executar o app.

```sh
openspec validate --all --strict
cargo test --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
```

Os testes de domínio, banco e relatórios podem rodar sem os pacotes de desenvolvimento GTK:

```sh
cargo test --no-default-features --locked
```

Para o teste da interface, instale também `xvfb` e `dbus-x11` e execute:

```sh
GDK_BACKEND=x11 GSK_RENDERER=cairo dbus-run-session -- xvfb-run -a cargo test --bin workcircuit --locked -- --ignored --test-threads=1
```

No GNOME/Wayland, confira a integração da minijanela com o compositor:

```sh
WORKCIRCUIT_WM_ONLY=1 GDK_BACKEND=wayland cargo test --bin workcircuit --locked -- --ignored --test-threads=1
```

A CI verifica formatação, Clippy, testes, interface sob Xvfb, build otimizado e instalação no Ubuntu 24.04. Para gerar um pacote local depois de compilar:

```sh
python3 scripts/package.py
```

O pacote inclui instalador, documentação nos dois idiomas, checksum SHA-256 e avisos das dependências. É necessário o componente `rust-docs` do Rust para incluir seus avisos. Use Ubuntu 24.04 para gerar o binário distribuído: compilar em uma distribuição mais nova pode exigir bibliotecas mais recentes. Os arquivos gerados ficam em `dist/`, ignorado pelo Git. Empacotadores podem usar `PREFIX=/usr` e `DESTDIR` na instalação.

Distribuição por APT/Snap, backup opcional na nuvem e sincronização entre PCs ficam para versões futuras.

## Licença

Nenhuma licença de reutilização é concedida para o WorkCircuit neste momento. A disponibilidade pública do código não concede permissão para reutilizá-lo, modificá-lo ou redistribuí-lo. Componentes de terceiros mantêm suas próprias licenças; os pacotes incluem seus avisos em `THIRD-PARTY-NOTICES.md` e `LICENSES/`.
