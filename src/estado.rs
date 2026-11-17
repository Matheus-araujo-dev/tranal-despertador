//! O estado do app: uma cópia só, compartilhada pela interface e pelo
//! agendador.
//!
//! Antes cada lado tinha a sua lista (a interface e a thread do agendador),
//! e isso desalinhavá: uma soneca criada na interface não existia no
//! agendador, e o índice do alarme disparado apontava para o item errado.
//! Agora os dois leem e escrevem o mesmo estado, protegido por um mutex.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};

use chrono::{DateTime, Local};

use crate::model::{Alarm, Settings};
use crate::schedule;
use crate::storage;

/// Estado compartilhado do app: os dados, o despertador e o sinal de parada.
pub struct Compartilhado {
    /// Os dados: despertadores do dono, sonecas da sessão e configurações.
    pub estado: Mutex<Estado>,
    /// Acorda o agendador quando o estado muda (ou quando o dono manda
    /// parar). O agendador dorme aqui em vez de ficar fazendo polling.
    pub condvar: Condvar,
    /// Quantas vezes o dono mandou parar o alarme. O agendador compara com o
    /// valor que tinha quando o alarme começou a tocar: se mudou, o dono
    /// dispensou e não vale a pena insistir.
    pub paradas: AtomicU64,
}

/// Os dados do app.
#[derive(Clone, Debug)]
pub struct Estado {
    /// Despertadores do dono. São gravados no arquivo de dados.
    pub alarms: Vec<Alarm>,
    /// Sonecas criadas nesta sessão. NÃO são gravadas.
    pub sonecas: Vec<Alarm>,
    /// Configurações do app.
    pub settings: Settings,
}

impl Estado {
    /// Estado novo, carregado do arquivo.
    pub fn new(alarms: Vec<Alarm>, settings: Settings) -> Self {
        Self {
            alarms,
            sonecas: Vec::new(),
            settings,
        }
    }

    /// Tudo que pode disparar: despertadores do dono e sonecas, nesta ordem.
    pub fn todos(&self) -> Vec<Alarm> {
        let mut todos = self.alarms.clone();
        todos.extend(self.sonecas.iter().cloned());
        todos
    }

    /// O próximo disparo depois de `agora`: o despertador e o horário.
    pub fn proximo_disparo(&self, agora: DateTime<Local>) -> Option<(Alarm, DateTime<Local>)> {
        let todos = self.todos();
        let (indice, quando) = schedule::proximo_disparo_global(&todos, agora)?;
        todos.get(indice).cloned().map(|a| (a, quando))
    }

    /// Despertador que já passou do horário e ainda não tocou hoje.
    ///
    /// Serve para pegar o alarme depois de o PC voltar da suspensão ou de o
    /// app ficar fechado por alguns minutos.
    pub fn atrasado(&self, agora: DateTime<Local>, janela_min: i64) -> Option<Alarm> {
        self.todos()
            .into_iter()
            .find(|a| schedule::atrasado_para_disparar(a, agora, janela_min))
    }

    /// Procura um despertador (do dono ou soneca) pelo id.
    pub fn por_id(&self, id: &str) -> Option<&Alarm> {
        self.alarms
            .iter()
            .chain(self.sonecas.iter())
            .find(|a| a.id == id)
    }

    /// Marca que o despertador tocou hoje.
    ///
    /// Vale para o despertador do dono e para a soneca: nenhum dos dois pode
    /// tocar de novo hoje.
    pub fn marcar_tocado(&mut self, id: &str, hoje: &str) {
        if let Some(a) = self.alarms.iter_mut().find(|a| a.id == id) {
            a.last_rung = Some(hoje.to_string());
            return;
        }
        if let Some(a) = self.sonecas.iter_mut().find(|a| a.id == id) {
            a.last_rung = Some(hoje.to_string());
        }
    }
}

impl Compartilhado {
    /// Estado compartilhado novo.
    pub fn novo(alarms: Vec<Alarm>, settings: Settings) -> Self {
        Self {
            estado: Mutex::new(Estado::new(alarms, settings)),
            condvar: Condvar::new(),
            paradas: AtomicU64::new(0),
        }
    }

    /// Trava o estado sem morrer se outra thread envenenou o mutex.
    pub fn travar(&self) -> MutexGuard<'_, Estado> {
        self.estado.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Acorda o agendador: o estado mudou e ele precisa recalcular a hora.
    pub fn notificar(&self) {
        self.condvar.notify_all();
    }

    /// O dono mandou parar o alarme: as tentativas em curso são abandonadas.
    pub fn pedir_parada(&self) {
        self.paradas.fetch_add(1, Ordering::SeqCst);
        self.condvar.notify_all();
    }

    /// Grava o arquivo de dados.
    ///
    /// Só os despertadores do dono entram no arquivo: a soneca é da sessão e
    /// some com ela.
    pub fn salvar(&self) -> Result<(), String> {
        let g = self.travar();
        storage::salvar(&g.alarms, &g.settings)
    }

    /// O dono dispensou o alarme: ele não toca de novo hoje.
    ///
    /// A soneca que estava tocando sai da fila. O despertador do dono fica
    /// marcado como atendido, para o agendador não tocar em cima da soneca.
    ///
    /// Também conta como "parada": o agendador pode estar no meio de uma
    /// re-tentativa de som e precisa saber que parou de insistir.
    pub fn dispensar(&self, id: &str) {
        let hoje = Local::now().format("%Y-%m-%d").to_string();
        {
            let mut g = self.travar();
            if let Some(a) = g.alarms.iter_mut().find(|a| a.id == id) {
                a.last_rung = Some(hoje);
            }
            g.sonecas.retain(|a| a.id != id);
        }
        self.pedir_parada();
        let _ = self.salvar();
    }

    /// Uma soneca nova entra na fila no lugar da que estava tocando.
    ///
    /// O despertador original já foi atendido (tocou ou o dono dispensou),
    /// então ele não toca de novo hoje: quem toca de novo é a soneca.
    ///
    /// Como na dispensa, isso conta como "parada": o agendador não pode
    /// insistir no som do alarme original por cima da soneca.
    pub fn adicionar_soneca(&self, soneca: Alarm, anterior_id: &str) {
        let hoje = Local::now().format("%Y-%m-%d").to_string();
        {
            let mut g = self.travar();
            g.sonecas.retain(|a| a.id != anterior_id);
            if let Some(a) = g.alarms.iter_mut().find(|a| a.id == anterior_id) {
                a.last_rung = Some(hoje);
            }
            g.sonecas.push(soneca);
        }
        self.pedir_parada();
        let _ = self.salvar();
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::model::RepeatRule;
    use chrono::TimeZone;

    fn em(ano: i32, mes: u32, dia: u32, hora: u32, minuto: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(ano, mes, dia, hora, minuto, 0).unwrap()
    }

    fn estado_com(alarms: Vec<Alarm>, sonecas: Vec<Alarm>) -> Estado {
        Estado {
            alarms,
            sonecas,
            settings: Settings::default(),
        }
    }

    #[test]
    fn todos_traz_despertadores_e_sonecas() {
        let alarme = Alarm::new(7, 0);
        let soneca = Alarm::new(7, 9);
        let estado = estado_com(vec![alarme], vec![soneca]);
        assert_eq!(estado.todos().len(), 2);
    }

    #[test]
    fn a_soneca_entra_no_proximo_disparo() {
        // Regressão: a soneca ficava fora da conta e nunca tocava.
        let mut alarme = Alarm::new(7, 0);
        alarme.repeat = RepeatRule::Daily;
        let mut soneca = Alarm::new(7, 9);
        soneca.repeat = RepeatRule::Once;
        let estado = estado_com(vec![alarme], vec![soneca.clone()]);

        let agora = em(2026, 11, 17, 7, 5);
        let (alvo, quando) = estado.proximo_disparo(agora).unwrap();
        assert_eq!(alvo.id, soneca.id, "a soneca é o próximo disparo");
        assert_eq!(quando, em(2026, 11, 17, 7, 9));
    }

    #[test]
    fn a_soneca_atrasada_e_pega_na_hora() {
        // O PC dormiu e a soneca ficou para trás: tem que tocar assim mesmo.
        let mut soneca = Alarm::new(7, 9);
        soneca.repeat = RepeatRule::Once;
        let estado = estado_com(Vec::new(), vec![soneca.clone()]);

        let agora = em(2026, 11, 17, 7, 12);
        let achado = estado.atrasado(agora, 15).unwrap();
        assert_eq!(achado.id, soneca.id);
    }

    #[test]
    fn por_id_acha_despertador_e_soneca() {
        let alarme = Alarm::new(7, 0);
        let soneca = Alarm::new(7, 9);
        let estado = estado_com(vec![alarme.clone()], vec![soneca.clone()]);
        assert_eq!(estado.por_id(&alarme.id).unwrap().id, alarme.id);
        assert_eq!(estado.por_id(&soneca.id).unwrap().id, soneca.id);
        assert!(estado.por_id("nao-existe").is_none());
    }

    #[test]
    fn marcar_tocado_marca_o_despertador_e_a_soneca() {
        let alarme = Alarm::new(7, 0);
        let soneca = Alarm::new(7, 9);
        let mut estado = estado_com(vec![alarme.clone()], vec![soneca.clone()]);
        estado.marcar_tocado(&alarme.id, "2026-11-17");
        estado.marcar_tocado(&soneca.id, "2026-11-17");
        assert_eq!(
            estado.por_id(&alarme.id).unwrap().last_rung.as_deref(),
            Some("2026-11-17")
        );
        assert_eq!(
            estado.por_id(&soneca.id).unwrap().last_rung.as_deref(),
            Some("2026-11-17")
        );
    }

    /// O agendador tem que acordar quando o estado muda.
    ///
    /// É o teste do padrão "condvar": quem dorme segura a trava desde o
    /// cálculo da próxima hora até a espera. Se a trava fosse solta entre as
    /// duas coisas, um aviso do dono nesse instante se perderia e o
    /// agendador dormiria até a hora antiga — o alarme novo, mais cedo, só
    /// tocaria horas depois.
    #[test]
    fn o_agendador_acorda_quando_o_estado_muda() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let comp = Arc::new(Compartilhado::novo(Vec::new(), Settings::default()));
        let acordou = Arc::new(AtomicBool::new(false));
        let marcador = acordou.clone();
        let outra = comp.clone();

        let agendador = std::thread::spawn(move || {
            // Mesmo caminho do agendador: trava, calcula, espera.
            let g = outra.travar();
            assert!(
                g.proximo_disparo(Local::now()).is_none(),
                "sem alarme, nao ha proximo disparo"
            );
            let _g = match outra.condvar.wait(g) {
                Ok(guardia) => guardia,
                Err(envenenado) => envenenado.into_inner(),
            };
            marcador.store(true, Ordering::SeqCst);
        });

        // O dono cadastra um despertador e avisa.
        std::thread::sleep(Duration::from_millis(150));
        {
            let mut g = comp.travar();
            g.alarms.push(Alarm::new(7, 0));
        }
        comp.notificar();

        agendador.join().unwrap();
        assert!(
            acordou.load(Ordering::SeqCst),
            "o agendador nao acordou quando o estado mudou"
        );
    }

    /// A soneca não pode entrar no arquivo de dados: ela é da sessão.
    #[test]
    fn a_soneca_nao_vai_para_o_arquivo() {
        let caminho = std::env::temp_dir().join("tranal-estado-teste.json");
        let alarme = Alarm::new(7, 0);
        let soneca = Alarm::new(7, 9);
        let estado = estado_com(vec![alarme.clone()], vec![soneca.clone()]);

        storage::salvar_em(&caminho, &estado.alarms, &estado.settings).unwrap();
        let texto = std::fs::read_to_string(&caminho).unwrap();
        assert!(texto.contains(&alarme.id), "o despertador vai para o arquivo");
        assert!(
            !texto.contains(&soneca.id),
            "a soneca NÃO pode ir para o arquivo"
        );
        let _ = std::fs::remove_file(&caminho);
    }
}
