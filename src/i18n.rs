use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "pt-BR")]
    PortugueseBrazil,
}

impl Language {
    pub fn text(self, english: &str) -> &str {
        if self == Self::English {
            return english;
        }
        match english {
            "Could not complete action" => "Não foi possível concluir a ação",
            "Close" => "Fechar",
            "Check your next task" => "Confira sua próxima tarefa",
            "You've reached your reminder time. Switch when you're ready." => {
                "Você atingiu o tempo do lembrete. Troque de tarefa quando quiser."
            }
            "Your work, in rotation" => "Seu trabalho, em rotação",
            "Open dashboard" => "Abrir painel",
            "Settings" => "Configurações",
            "Overlay" => "Minijanela",
            "Reports" => "Relatórios",
            "Quit" => "Sair",
            "CURRENT TASK" => "TAREFA ATUAL",
            "Start" => "Iniciar",
            "Next →" => "Próxima →",
            "Task rotation" => "Rotação de tarefas",
            "Add Task" => "Adicionar tarefa",
            "Add a task to get started" => "Adicione uma tarefa para começar",
            "Scheduled break · resume afterwards" => "Intervalo agendado · retome depois",
            "Up next:" => "A seguir:",
            "Pause" => "Pausar",
            "Resume" => "Retomar",
            "No active tasks yet. Add your first task in the dashboard." => {
                "Nenhuma tarefa ativa. Adicione sua primeira tarefa no painel."
            }
            "Select task" => "Selecionar tarefa",
            "Move up" => "Mover para cima",
            "Move down" => "Mover para baixo",
            "Edit task" => "Editar tarefa",
            "Archive task (preserves history)" => "Arquivar tarefa (preserva o histórico)",
            "Archived tasks" => "Tarefas arquivadas",
            "Restore" => "Restaurar",
            "Edit Task" => "Editar tarefa",
            "Task title (required)" => "Título da tarefa (obrigatório)",
            "Description (optional)" => "Descrição (opcional)",
            "Save Task" => "Salvar tarefa",
            "Reminder" => "Lembrete",
            "Minutes per task visit. 0 disables reminders. Breaks preserve elapsed work." => {
                "Minutos por passagem na tarefa. 0 desativa lembretes. Pausas preservam o tempo trabalhado."
            }
            "Remind after (minutes)" => "Lembrar após (minutos)",
            "Scheduled breaks" => "Intervalos agendados",
            "Local computer time. Overnight ranges belong to their starting weekday. Resume manually afterwards." => {
                "Horário local do computador. Intervalos noturnos pertencem ao dia em que começam. Retome manualmente depois."
            }
            "Add Scheduled Break" => "Adicionar intervalo agendado",
            "Save Settings" => "Salvar configurações",
            "No scheduled breaks" => "Nenhum intervalo agendado",
            "Mon" => "Seg",
            "Tue" => "Ter",
            "Wed" => "Qua",
            "Thu" => "Qui",
            "Fri" => "Sex",
            "Sat" => "Sáb",
            "Sun" => "Dom",
            "Remove scheduled break" => "Remover intervalo agendado",
            "Scheduled Break" => "Intervalo agendado",
            "Start HH:MM" => "Início HH:MM",
            "End HH:MM" => "Fim HH:MM",
            "Start / end in 24-hour time" => "Início / fim no formato de 24 horas",
            "Add Break" => "Adicionar intervalo",
            "From YYYY-MM-DD" => "De AAAA-MM-DD",
            "Through YYYY-MM-DD" => "Até AAAA-MM-DD",
            "Show" => "Exibir",
            "From / through (inclusive local dates, YYYY-MM-DD)" => {
                "De / até (datas locais inclusivas, AAAA-MM-DD)"
            }
            "Total · 00:00:00" => "Total · 00:00:00",
            "Export Totals CSV" => "Exportar totais CSV",
            "Export Sessions CSV" => "Exportar sessões CSV",
            "No tracked time in this period" => "Nenhum tempo registrado neste período",
            "Session history" => "Histórico de sessões",
            "Export CSV" => "Exportar CSV",
            "in period" => "no período",
            "Language" => "Idioma",
            "Choose your interface language. English is the default." => {
                "Escolha o idioma da interface. O padrão é inglês."
            }
            "Choose task" => "Escolher tarefa",
            "Close overlay" => "Fechar minijanela",
            "Ready when you are" => "Tudo pronto para começar",
            "Tracking" => "Contando tempo",
            "Paused — resume when ready" => "Pausado — retome quando quiser",
            "Paused for system sleep" => "Pausado pela suspensão do sistema",
            "Session ended" => "Sessão encerrada",
            "Task archived" => "Tarefa arquivada",
            "Scheduled break — resume manually afterwards" => {
                "Intervalo agendado — retome manualmente depois"
            }
            "Scheduled break — resume after it ends" => {
                "Intervalo agendado — retome quando terminar"
            }
            "Paused after sleep or an interrupted timer" => {
                "Pausado após suspensão ou interrupção do cronômetro"
            }
            "Time to check your next task" => "Hora de conferir a próxima tarefa",
            "Recovered interrupted work at the last saved checkpoint" => {
                "Trabalho interrompido recuperado no último registro salvo"
            }
            "Storage error — tracking paused" => "Erro ao salvar — contagem pausada",
            "A task title is required" => "O título da tarefa é obrigatório",
            "Task not found" => "Tarefa não encontrada",
            "Task is not in the active queue" => "A tarefa não está na fila ativa",
            "Add a task first" => "Adicione uma tarefa primeiro",
            "Add or select a task first" => "Adicione ou selecione uma tarefa primeiro",
            "Reminder must be between 0 and 1440 minutes" => {
                "O lembrete deve estar entre 0 e 1440 minutos"
            }
            "Select at least one valid weekday" => "Selecione pelo menos um dia da semana",
            "Break start and end must differ" => {
                "O início e o fim do intervalo devem ser diferentes"
            }
            "End date must not precede start date" => {
                "A data final não pode ser anterior à inicial"
            }
            "Always-on-top needs X11/XWayland. Install or enable XWayland and reopen the overlay." => {
                "A fixação precisa do X11/XWayland. Instale ou ative o XWayland e abra a minijanela novamente."
            }
            _ => english,
        }
    }
}
