/*
===============================================================================
                       SPELL COACH IA - MÓDULO CENTRAL
===============================================================================
Este arquivo (mod.rs) funciona como a "Portaria" ou o "Painel de Controle" do
nosso serviço de Coach. 

Para um estudante de programação:
No Rust, quando dividimos um arquivo grande em vários arquivos menores (submódulos),
por padrão eles ficam isolados e invisíveis para o resto do sistema. 

Para resolver isso de forma elegante (Clean Code), nós:
1. Declaramos os submódulos usando 'pub mod <nome>;'
2. Re-exportamos tudo o que há de público dentro deles usando 'pub use <nome>::*;'

Benefício: Qualquer outro arquivo do projeto (como o 'lib.rs' ou o 'bridge.rs')
pode continuar importando 'crate::db::coach_service::minha_funcao' sem precisar
saber que dividimos o arquivo original. Isso se chama Encapsulamento de Fachada.
===============================================================================
*/

// 1. DECLARAÇÃO DOS SUBMÓDULOS (Os arquivos físicos na mesma pasta)
pub mod heuristics; // Regras de viabilidade, ordenações matemáticas e estatísticas
pub mod runes;      // Recomendador de runas, explicações didáticas e feitiços
pub mod tactical;   // Telemetria em tempo real (Riot LCA) e inteligência tática/compras
pub mod stats;      // Consultas SQL, histórico do banco, builds core e counters

// 2. RE-EXPORTAÇÃO GLOBAL (Expõe as funções públicas no escopo do coach_service)
// O operador '*' (wildcard) traz tudo o que foi marcado com 'pub' nos submódulos.
// Isso inclui structs, enums e as funções geradas pela macro #[tauri::command].
pub use heuristics::*;
pub use runes::*;
pub use tactical::*;
pub use stats::*;
