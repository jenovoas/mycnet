// Autor: Jaime Novoa Sepulveda — Todos los derechos reservados.
// Licencia: Apache 2.0 + Cláusula No Comercial (ver LICENSE).
//! # predict — Ventana de envío por coherencia no-Markoviana (OU en S60)
//!
//! Port fiel de `me60os_core::buffer::BufferCascade` (OU + historial 68)
//! a `mycnet-core`, sin dependencia del core pesado (DMZ solo-mycnetd).
//! Misma matemática: kernel OU, decaimiento 5/6, ventana de historial.
//! Uso: `mycnetd` registra `tq_s60` por peer y predice si enviar ahora
//! o retener en el ring local hasta que la ventana mejore.

use crate::s60::SPA;
use std::collections::BTreeMap;

/// Kernel Ornstein-Uhlenbeck en S60: K(t,s) = (1/2τ)·exp(-|t-s|/τ).
/// Serie Taylor 15 términos en i128, idéntica a `SPAMath::exp`.
fn ou_kernel(t: SPA, s: SPA, tau: SPA) -> SPA {
    let dt = if t > s { t - s } else { s - t };
    let two_tau = SPA::new(2, 0, 0, 0, 0) * tau;
    if two_tau.to_raw() == 0 {
        return SPA::zero();
    }
    let prefactor = SPA::new(1, 0, 0, 0, 0) / two_tau;
    let exponent = -(dt / tau);
    prefactor * spa_exp(exponent)
}

/// exp S60 por Taylor (15 términos, i128). Idéntica a SPAMath::exp.
fn spa_exp(x: SPA) -> SPA {
    let x_raw = x.to_raw();
    if x_raw == 0 {
        return SPA::new(1, 0, 0, 0, 0);
    }
    let mut res = SPA::SCALE_0 as i128;
    let mut term = SPA::SCALE_0 as i128;
    for i in 1..15 {
        term = (term * x_raw as i128) / (i as i128 * SPA::SCALE_0 as i128);
        if term == 0 {
            break;
        }
        res += term;
    }
    SPA::from_raw(res as i64)
}
/// Ventana de envío por peer: historial TQ + predicción OU.
#[derive(Clone, Debug)]
pub struct SendWindow {
    history: BTreeMap<i64, SPA>,
    tau: SPA,
    limit: SPA,
}

impl SendWindow {
    pub fn new() -> Self {
        Self {
            history: BTreeMap::new(),
            // τ = 1.0 en S60. Límite 58 (igual que BufferCascade::with_default_hex).
            tau: SPA::new(1, 0, 0, 0, 0),
            limit: SPA::new(58, 0, 0, 0, 0),
        }
    }

    /// Registra coherencia observada del peer (tq_s60) en el tick dado.
    pub fn record(&mut self, tick: i64, coherence: SPA) {
        self.history.insert(tick, coherence);
        // Ventana 68 ticks (1 ciclo Salto-17, igual que BufferCascade/PID).
        while self.history.len() > 68 {
            self.history.pop_first();
        }
    }

    /// Predice coherencia futura del peer. Si >= límite, enviar ahora;
    /// si no, retener en ring local.
    pub fn should_send(&self, now_tick: i64, window: usize) -> bool {
        self.predicted(now_tick, window) >= self.limit
    }

    /// Coherencia predicha por kernel OU sobre historial reciente.
    pub fn predicted(&self, now_tick: i64, window: usize) -> SPA {
        let now = SPA::from_int(now_tick);
        let unit = SPA::from_int(1);
        let mut acc = SPA::zero();
        for (tick, coh) in self.history.iter().rev().take(window) {
            let t = SPA::from_int(*tick);
            let kernel = ou_kernel(now, t, self.tau);
            let weight = if self.limit == SPA::zero() {
                unit
            } else {
                *coh / self.limit
            };
            acc = acc + (kernel * weight * SPA::new(0, 30, 0, 0, 0));
        }
        // Base 42;30 + memoria×22, clamp a límite (igual que cascade()).
        let base = SPA::new(42, 30, 0, 0, 0);
        let boosted = base + (acc * SPA::new(22, 0, 0, 0, 0));
        if boosted < self.limit {
            boosted
        } else {
            self.limit
        }
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

impl Default for SendWindow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_predicts_high_coherence_peer() {
        let mut w = SendWindow::new();
        // Peer estable en TQ 255 (= 1.0) durante 10 ticks.
        for t in 0..10 {
            w.record(t, SPA::one());
        }
        assert!(w.should_send(10, 10));
    }

    #[test]
    fn test_empty_window_does_not_send() {
        let w = SendWindow::new();
        assert!(!w.should_send(0, 10));
    }
}
