extern crate alloc;

use crate::utils;
use alloc::vec;
use alloc::vec::Vec;
use reed_solomon_erasure::galois_16::ReedSolomon;
use subspace_core_primitives::crypto::kzg::{Kzg, Polynomial};
use subspace_core_primitives::crypto::{blake2b_256_254_hash_to_scalar, Scalar};
use subspace_core_primitives::{FlatPieces, Piece, PIECES_IN_SEGMENT, RECORD_SIZE};

/// Reconstructor-related instantiation error.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum ReconstructorInstantiationError {
    /// Segment size is not bigger than record size
    #[cfg_attr(
        feature = "thiserror",
        error("Segment size is not bigger than record size")
    )]
    SegmentSizeTooSmall,
    /// Segment size is not a multiple of record size
    #[cfg_attr(
        feature = "thiserror",
        error("Segment size is not a multiple of record size")
    )]
    SegmentSizesNotMultipleOfRecordSize,
}

/// Reconstructor-related instantiation error
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum ReconstructorError {
    /// Segment size is not bigger than record size
    #[cfg_attr(
        feature = "thiserror",
        error("Error during data shards reconstruction: {0}")
    )]
    DataShardsReconstruction(reed_solomon_erasure::Error),

    /// Incorrect piece position provided.
    #[cfg_attr(feature = "thiserror", error("Incorrect piece position provided."))]
    IncorrectPiecePosition,
}

/// Reconstructor helps to retrieve blocks from archived pieces.
#[derive(Debug, Clone)]
pub struct PiecesReconstructor {
    /// Number of data shards
    source_shards: u32,
    /// Number of parity shards
    parity_shards: u32,
    /// Erasure coding data structure
    reed_solomon: ReedSolomon,
    /// KZG instance
    kzg: Kzg,
}

impl PiecesReconstructor {
    pub fn new(segment_size: u32, kzg: Kzg) -> Result<Self, ReconstructorInstantiationError> {
        if segment_size <= RECORD_SIZE {
            return Err(ReconstructorInstantiationError::SegmentSizeTooSmall);
        }
        if segment_size % RECORD_SIZE != 0 {
            return Err(ReconstructorInstantiationError::SegmentSizesNotMultipleOfRecordSize);
        }

        let source_shards = segment_size / RECORD_SIZE;
        let parity_shards = source_shards;
        let reed_solomon = ReedSolomon::new(source_shards as usize, parity_shards as usize)
            .expect("ReedSolomon must always be correctly instantiated");

        Ok(Self {
            source_shards,
            parity_shards,
            reed_solomon,
            kzg,
        })
    }

    /// Returns incomplete pieces (witness missing) and polynomial that can be used to generate
    /// necessary witnesses later.
    fn reconstruct_shards(
        &self,
        segment_pieces: &[Option<Piece>],
    ) -> Result<(FlatPieces, Polynomial), ReconstructorError> {
        // If not all data pieces are available, need to reconstruct data shards using erasure
        // coding.
        let mut shards = segment_pieces
            .iter()
            .map(|maybe_piece| {
                maybe_piece
                    .as_ref()
                    .map(|piece| utils::slice_to_arrays(&piece.record()))
            })
            .collect::<Vec<_>>();

        self.reed_solomon
            .reconstruct(&mut shards)
            .map_err(ReconstructorError::DataShardsReconstruction)?;

        let mut reconstructed_record_shards = FlatPieces::new(shards.len());
        let mut record_commitments =
            vec![Scalar::default(); (self.source_shards + self.parity_shards) as usize];
        //TODO: Parity hashes will be erasure coded instead in the future
        //TODO: reuse already present commitments from segment_pieces, so we don't re-derive what
        // we already have
        reconstructed_record_shards
            .as_pieces_mut()
            .zip(record_commitments.iter_mut())
            .zip(shards)
            .for_each(|((mut piece, polynomial_data), record)| {
                let record =
                    record.expect("Reconstruction just happened and all records are present; qed");
                let record = record.flatten();
                piece.record_mut().as_mut().copy_from_slice(record);
                *polynomial_data = blake2b_256_254_hash_to_scalar(record);
            });

        let polynomial = self
            .kzg
            .poly(&record_commitments)
            .expect("Internally produced values must never fail; qed");

        Ok((reconstructed_record_shards, polynomial))
    }

    /// Returns all the pieces for a segment using given set of pieces of a segment of the archived
    /// history (any half of all pieces are required to be present, the rest will be recovered
    /// automatically due to use of erasure coding if needed).
    pub fn reconstruct_segment(
        &self,
        segment_pieces: &[Option<Piece>],
    ) -> Result<FlatPieces, ReconstructorError> {
        let (mut pieces, polynomial) = self.reconstruct_shards(segment_pieces)?;

        pieces
            .as_pieces_mut()
            .enumerate()
            .for_each(|(position, mut piece)| {
                piece.witness_mut().as_mut().copy_from_slice(
                    &self
                        .kzg
                        .create_witness(&polynomial, position as u32)
                        // TODO: Update this proof here and in other places, we don't use Merkle
                        //  trees anymore
                        .expect("We use the same indexes as during Merkle tree creation; qed")
                        .to_bytes(),
                );
            });

        Ok(pieces)
    }

    /// Returns the missing piece for a segment using given set of pieces of a segment of the archived
    /// history (any half of all pieces are required to be present).
    pub fn reconstruct_piece(
        &self,
        segment_pieces: &[Option<Piece>],
        piece_position: usize, // piece position within the segment (offset)
    ) -> Result<Piece, ReconstructorError> {
        let (reconstructed_records, polynomial) = self.reconstruct_shards(segment_pieces)?;

        if piece_position >= PIECES_IN_SEGMENT as usize {
            return Err(ReconstructorError::IncorrectPiecePosition);
        }

        let mut piece = Piece::from(
            reconstructed_records
                .as_pieces()
                .nth(piece_position)
                .expect(
                "Piece exists at the position within segment after successful reconstruction; qed",
            ),
        );

        piece.witness_mut().as_mut().copy_from_slice(
            &self
                .kzg
                .create_witness(&polynomial, piece_position as u32)
                .expect("We use the same indexes as during Merkle tree creation; qed")
                .to_bytes(),
        );

        Ok(piece)
    }
}
