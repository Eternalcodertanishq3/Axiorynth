use std::fs::File;
use std::io::{Read, Write, Result};
use crate::board::Board;
use crate::types::Color;
use crate::eval::get_half_kp_features;

pub const FEATURE_SIZE: usize = 40960;
pub const L1_SIZE: usize = 256;
pub const L2_SIZE: usize = 32;

#[derive(Clone)]
pub struct NnueNetwork {
    pub w1: Vec<f32>, // FEATURE_SIZE * L1_SIZE
    pub b1: Vec<f32>, // L1_SIZE
    pub w2: Vec<f32>, // (L1_SIZE * 2) * L2_SIZE (512 * 32)
    pub b2: Vec<f32>, // L2_SIZE
    pub w3: Vec<f32>, // L2_SIZE
    pub b3: f32,
}

impl NnueNetwork {
    pub fn new_random() -> Self {
        // Deterministic pseudo-random initialization using LCG
        let mut seed = 42u64;
        let mut next_rand = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 32) as u32
        };

        let mut next_f32 = |scale: f32| -> f32 {
            let r = next_rand() as f32 / u32::MAX as f32;
            (r * 2.0 - 1.0) * scale
        };

        let w1_size = FEATURE_SIZE * L1_SIZE;
        let mut w1 = vec![0.0f32; w1_size];
        let scale1 = (2.0 / FEATURE_SIZE as f32).sqrt();
        for val in w1.iter_mut() {
            *val = next_f32(scale1);
        }

        let mut b1 = vec![0.0f32; L1_SIZE];
        for val in b1.iter_mut() {
            *val = next_f32(0.01);
        }

        let w2_size = (L1_SIZE * 2) * L2_SIZE;
        let mut w2 = vec![0.0f32; w2_size];
        let scale2 = (2.0 / (L1_SIZE * 2) as f32).sqrt();
        for val in w2.iter_mut() {
            *val = next_f32(scale2);
        }

        let mut b2 = vec![0.0f32; L2_SIZE];
        for val in b2.iter_mut() {
            *val = next_f32(0.01);
        }

        let mut w3 = vec![0.0f32; L2_SIZE];
        let scale3 = (2.0 / L2_SIZE as f32).sqrt();
        for val in w3.iter_mut() {
            *val = next_f32(scale3);
        }

        let b3 = next_f32(0.01);

        Self { w1, b1, w2, b2, w3, b3 }
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let mut file = File::create(path)?;
        
        // Helper to write a float slice as bytes
        fn write_floats(file: &mut File, slice: &[f32]) -> Result<()> {
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    slice.as_ptr() as *const u8,
                    slice.len() * std::mem::size_of::<f32>(),
                )
            };
            file.write_all(bytes)
        }

        write_floats(&mut file, &self.w1)?;
        write_floats(&mut file, &self.b1)?;
        write_floats(&mut file, &self.w2)?;
        write_floats(&mut file, &self.b2)?;
        write_floats(&mut file, &self.w3)?;
        
        let b3_bytes = self.b3.to_ne_bytes();
        file.write_all(&b3_bytes)?;

        Ok(())
    }

    pub fn load(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        
        let w1_len = FEATURE_SIZE * L1_SIZE;
        let mut w1 = vec![0.0f32; w1_len];
        let b1_len = L1_SIZE;
        let mut b1 = vec![0.0f32; b1_len];
        let w2_len = (L1_SIZE * 2) * L2_SIZE;
        let mut w2 = vec![0.0f32; w2_len];
        let b2_len = L2_SIZE;
        let mut b2 = vec![0.0f32; b2_len];
        let w3_len = L2_SIZE;
        let mut w3 = vec![0.0f32; w3_len];

        fn read_floats(file: &mut File, slice: &mut [f32]) -> Result<()> {
            let bytes: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(
                    slice.as_mut_ptr() as *mut u8,
                    slice.len() * std::mem::size_of::<f32>(),
                )
            };
            file.read_exact(bytes)
        }

        read_floats(&mut file, &mut w1)?;
        read_floats(&mut file, &mut b1)?;
        read_floats(&mut file, &mut w2)?;
        read_floats(&mut file, &mut b2)?;
        read_floats(&mut file, &mut w3)?;

        let mut b3_bytes = [0u8; 4];
        file.read_exact(&mut b3_bytes)?;
        let b3 = f32::from_ne_bytes(b3_bytes);

        Ok(Self { w1, b1, w2, b2, w3, b3 })
    }

    /// Forward pass through the network.
    /// Returns (output, internal_state) where internal_state is used for training.
    pub fn forward(&self, white_features: &[usize], black_features: &[usize]) -> (f32, ForwardState) {
        // Step 1: Compute accumulators (Layer 1)
        let mut acc_w = self.b1.clone();
        let mut acc_b = self.b1.clone();

        for &feat in white_features {
            if feat < FEATURE_SIZE {
                let offset = feat * L1_SIZE;
                for i in 0..L1_SIZE {
                    acc_w[i] += self.w1[offset + i];
                }
            }
        }

        for &feat in black_features {
            if feat < FEATURE_SIZE {
                let offset = feat * L1_SIZE;
                for i in 0..L1_SIZE {
                    acc_b[i] += self.w1[offset + i];
                }
            }
        }

        // Apply activation (Clipped ReLU: clamp to [0.0, 1.0])
        let mut act_acc_w = vec![0.0f32; L1_SIZE];
        let mut act_acc_b = vec![0.0f32; L1_SIZE];
        for i in 0..L1_SIZE {
            act_acc_w[i] = acc_w[i].clamp(0.0, 1.0);
            act_acc_b[i] = acc_b[i].clamp(0.0, 1.0);
        }

        // Concatenate white and black perspectives
        let mut layer1_output = vec![0.0f32; L1_SIZE * 2];
        layer1_output[0..L1_SIZE].copy_from_slice(&act_acc_w);
        layer1_output[L1_SIZE..L1_SIZE * 2].copy_from_slice(&act_acc_b);

        // Step 2: Hidden layer (Layer 2)
        let mut acc_l2 = self.b2.clone();
        for j in 0..L2_SIZE {
            let offset = j * (L1_SIZE * 2);
            let mut sum = 0.0f32;
            for i in 0..(L1_SIZE * 2) {
                sum += layer1_output[i] * self.w2[offset + i];
            }
            acc_l2[j] += sum;
        }

        let mut act_acc_l2 = vec![0.0f32; L2_SIZE];
        for j in 0..L2_SIZE {
            act_acc_l2[j] = acc_l2[j].clamp(0.0, 1.0);
        }

        // Step 3: Output layer (Layer 3)
        let mut output = self.b3;
        for j in 0..L2_SIZE {
            output += act_acc_l2[j] * self.w3[j];
        }

        (
            output,
            ForwardState {
                acc_w,
                acc_b,
                act_acc_w,
                act_acc_b,
                acc_l2,
                act_acc_l2,
            },
        )
    }

    /// Evaluates the position from the perspective of the side to move.
    pub fn evaluate_board(&self, board: &Board) -> i32 {
        let side = board.side_to_move();
        let white_features = get_half_kp_features(board, Color::White);
        let black_features = get_half_kp_features(board, Color::Black);

        let (score, _) = if side == Color::White {
            self.forward(&white_features, &black_features)
        } else {
            // Note: Perspective from black means we swap the feature order
            self.forward(&black_features, &white_features)
        };

        // Network outputs score in centipawns
        score.round() as i32
    }

    /// Trains the network on a batch of training examples.
    /// Each example is (white_features, black_features, target_score_from_white_perspective)
    pub fn train_batch(&mut self, batch: &[(Vec<usize>, Vec<usize>, f32)], lr: f32) -> f32 {
        let mut total_loss = 0.0f32;
        
        // Accumulators for gradients
        let mut dw1 = vec![0.0f32; FEATURE_SIZE * L1_SIZE];
        let mut db1 = vec![0.0f32; L1_SIZE];
        let mut dw2 = vec![0.0f32; (L1_SIZE * 2) * L2_SIZE];
        let mut db2 = vec![0.0f32; L2_SIZE];
        let mut dw3 = vec![0.0f32; L2_SIZE];
        let mut db3 = 0.0f32;

        for &(ref w_feat, ref b_feat, target) in batch {
            let (output, state) = self.forward(w_feat, b_feat);
            let error = output - target;
            total_loss += error * error;

            // Gradient at output: dLoss/dOutput
            let d_out = 2.0 * error / batch.len() as f32;

            // Layer 3 gradients
            db3 += d_out;
            for j in 0..L2_SIZE {
                dw3[j] += d_out * state.act_acc_l2[j];
            }

            // Backprop through Layer 3
            let mut d_act_l2 = vec![0.0f32; L2_SIZE];
            for j in 0..L2_SIZE {
                d_act_l2[j] = d_out * self.w3[j];
            }

            // Backprop through activation of L2
            let mut d_l2 = vec![0.0f32; L2_SIZE];
            for j in 0..L2_SIZE {
                if state.acc_l2[j] > 0.0 && state.acc_l2[j] < 1.0 {
                    d_l2[j] = d_act_l2[j];
                }
            }

            // Layer 2 gradients
            for j in 0..L2_SIZE {
                db2[j] += d_l2[j];
                let offset = j * (L1_SIZE * 2);
                
                // White features part of Layer 1 output
                for i in 0..L1_SIZE {
                    dw2[offset + i] += d_l2[j] * state.act_acc_w[i];
                }
                // Black features part of Layer 1 output
                for i in 0..L1_SIZE {
                    dw2[offset + L1_SIZE + i] += d_l2[j] * state.act_acc_b[i];
                }
            }

            // Backprop through Layer 2 to Layer 1 output
            let mut d_act_l1 = vec![0.0f32; L1_SIZE * 2];
            for j in 0..L2_SIZE {
                let offset = j * (L1_SIZE * 2);
                for i in 0..(L1_SIZE * 2) {
                    d_act_l1[i] += d_l2[j] * self.w2[offset + i];
                }
            }

            // Backprop through activation of L1
            let mut d_acc_w = vec![0.0f32; L1_SIZE];
            let mut d_acc_b = vec![0.0f32; L1_SIZE];
            for i in 0..L1_SIZE {
                if state.acc_w[i] > 0.0 && state.acc_w[i] < 1.0 {
                    d_acc_w[i] = d_act_l1[i];
                }
                if state.acc_b[i] > 0.0 && state.acc_b[i] < 1.0 {
                    d_acc_b[i] = d_act_l1[L1_SIZE + i];
                }
            }

            // Layer 1 gradients
            for i in 0..L1_SIZE {
                db1[i] += d_acc_w[i] + d_acc_b[i];
            }

            for &feat in w_feat {
                if feat < FEATURE_SIZE {
                    let offset = feat * L1_SIZE;
                    for i in 0..L1_SIZE {
                        dw1[offset + i] += d_acc_w[i];
                    }
                }
            }

            for &feat in b_feat {
                if feat < FEATURE_SIZE {
                    let offset = feat * L1_SIZE;
                    for i in 0..L1_SIZE {
                        dw1[offset + i] += d_acc_b[i];
                    }
                }
            }
        }

        // Apply updates (simple gradient descent with clipping to avoid explosion)
        let grad_clip = 10.0f32;
        let clip = |g: f32| -> f32 { g.clamp(-grad_clip, grad_clip) };

        for i in 0..(FEATURE_SIZE * L1_SIZE) {
            self.w1[i] -= lr * clip(dw1[i]);
        }
        for i in 0..L1_SIZE {
            self.b1[i] -= lr * clip(db1[i]);
        }
        for i in 0..((L1_SIZE * 2) * L2_SIZE) {
            self.w2[i] -= lr * clip(dw2[i]);
        }
        for i in 0..L2_SIZE {
            self.b2[i] -= lr * clip(db2[i]);
        }
        for i in 0..L2_SIZE {
            self.w3[i] -= lr * clip(dw3[i]);
        }
        self.b3 -= lr * clip(db3);

        total_loss / batch.len() as f32
    }
}

pub struct ForwardState {
    pub acc_w: Vec<f32>,
    pub acc_b: Vec<f32>,
    pub act_acc_w: Vec<f32>,
    pub act_acc_b: Vec<f32>,
    pub acc_l2: Vec<f32>,
    pub act_acc_l2: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nnue_initialization_and_forward() {
        let net = NnueNetwork::new_random();
        
        let white_features = vec![0, 50, 1000];
        let black_features = vec![1, 60, 2000];
        
        let (output, state) = net.forward(&white_features, &black_features);
        
        // Assert outputs and state are reasonable
        assert!(!output.is_nan());
        assert_eq!(state.acc_w.len(), L1_SIZE);
        assert_eq!(state.acc_b.len(), L1_SIZE);
        assert_eq!(state.act_acc_w.len(), L1_SIZE);
        assert_eq!(state.act_acc_b.len(), L1_SIZE);
        assert_eq!(state.acc_l2.len(), L2_SIZE);
        assert_eq!(state.act_acc_l2.len(), L2_SIZE);
        
        // Check activation range (Clipped ReLU)
        for &val in &state.act_acc_w {
            assert!(val >= 0.0 && val <= 1.0);
        }
        for &val in &state.act_acc_b {
            assert!(val >= 0.0 && val <= 1.0);
        }
        for &val in &state.act_acc_l2 {
            assert!(val >= 0.0 && val <= 1.0);
        }
    }

    #[test]
    fn test_nnue_save_load() {
        let net = NnueNetwork::new_random();
        let path = "test_axiorynth.nnue";
        
        net.save(path).unwrap();
        let loaded = NnueNetwork::load(path).unwrap();
        
        // Verify weights are exactly equal
        assert_eq!(net.b3, loaded.b3);
        assert_eq!(net.b2, loaded.b2);
        assert_eq!(net.w3, loaded.w3);
        
        // Verify values
        let white_features = vec![10, 20];
        let black_features = vec![30, 40];
        let (out1, _) = net.forward(&white_features, &black_features);
        let (out2, _) = loaded.forward(&white_features, &black_features);
        assert_eq!(out1, out2);
        
        // Cleanup
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_nnue_training_reduces_loss() {
        let mut net = NnueNetwork::new_random();
        
        // Let's create a small dummy batch of 2 examples
        let batch = vec![
            (vec![12, 100], vec![15, 200], 150.0f32),
            (vec![150, 400], vec![30, 80], -50.0f32),
        ];
        
        let (out_init_0, _) = net.forward(&batch[0].0, &batch[0].1);
        let (out_init_1, _) = net.forward(&batch[1].0, &batch[1].1);
        
        let init_loss = (out_init_0 - 150.0).powi(2) + (out_init_1 - (-50.0)).powi(2);
        
        // Train for 10 iterations on this batch
        let mut last_loss = 0.0;
        for _ in 0..10 {
            last_loss = net.train_batch(&batch, 0.05);
        }
        
        let (out_final_0, _) = net.forward(&batch[0].0, &batch[0].1);
        let (out_final_1, _) = net.forward(&batch[1].0, &batch[1].1);
        
        let final_loss = (out_final_0 - 150.0).powi(2) + (out_final_1 - (-50.0)).powi(2);
        
        // Loss should have decreased significantly
        assert!(final_loss < init_loss, "Init loss: {}, Final loss: {}", init_loss, final_loss);
        assert!(last_loss < init_loss / 2.0);
    }
}
