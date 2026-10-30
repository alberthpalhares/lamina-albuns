//! Guarda valores com limite em bytes (spec LV, regra 4): quando passa do limite, saem os usados
//! há mais tempo. Serve para imagens em memória, onde contar itens não diz quanto se gasta.

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
struct Slot<V> {
    value: V,
    bytes: usize,
    used: u64,
}

/// Os valores guardados, com a soma dos tamanhos sempre até `limit`.
#[derive(Debug)]
pub struct ByteCache<K, V> {
    slots: HashMap<K, Slot<V>>,
    limit: usize,
    bytes: usize,
    clock: u64,
}

impl<K: Eq + Hash + Clone, V: Clone> ByteCache<K, V> {
    pub fn new(limit: usize) -> Self {
        ByteCache {
            slots: HashMap::new(),
            limit,
            bytes: 0,
            clock: 0,
        }
    }

    /// O valor guardado em `key`, que passa a ser o usado mais recentemente.
    pub fn get(&mut self, key: &K) -> Option<V> {
        self.clock += 1;
        let slot = self.slots.get_mut(key)?;
        slot.used = self.clock;
        Some(slot.value.clone())
    }

    /// Guarda `value`, de `bytes` bytes. Um valor maior que o limite não é guardado.
    pub fn insert(&mut self, key: K, value: V, bytes: usize) {
        self.remove(&key);
        if bytes > self.limit {
            return;
        }
        self.clock += 1;
        let used = self.clock;
        self.bytes += bytes;
        self.slots.insert(key, Slot { value, bytes, used });
        while self.bytes > self.limit {
            self.remove_oldest();
        }
    }

    pub fn remove(&mut self, key: &K) {
        if let Some(slot) = self.slots.remove(key) {
            self.bytes -= slot.bytes;
        }
    }

    fn remove_oldest(&mut self) {
        let oldest = self.slots.iter().min_by_key(|(_, slot)| slot.used);
        if let Some(key) = oldest.map(|(key, _)| key.clone()) {
            self.remove(&key);
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.bytes = 0;
    }

    /// Quantos bytes estão guardados agora.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn lv_b6_passando_do_limite_saem_os_usados_ha_mais_tempo() {
        let mut cache = ByteCache::new(100);
        cache.insert("a", 1, 40);
        cache.insert("b", 2, 40);
        assert_eq!(cache.get(&"a"), Some(1));
        cache.insert("c", 3, 40);
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"c"), Some(3));
        assert_eq!(cache.bytes(), 80);
    }

    #[test]
    fn lv_b6_valor_maior_que_o_limite_nao_fica() {
        let mut cache = ByteCache::new(10);
        cache.insert("a", 1, 11);
        assert!(cache.is_empty());
        assert_eq!(cache.bytes(), 0);
    }

    #[test]
    fn lv_b7_esvaziar_zera_os_bytes() {
        let mut cache = ByteCache::new(10);
        cache.insert("a", 1, 5);
        cache.insert("a", 2, 7);
        assert_eq!(cache.bytes(), 7);
        cache.clear();
        assert_eq!((cache.len(), cache.bytes()), (0, 0));
    }
}
