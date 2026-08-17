params.samplesheet = "${workflow.projectDir}/run_params/samplesheets/samplesheet.csv"

include { compare_bpcsr_readers as COMPARE_READERS } from './compare_readers.nf'
include { compare_embl_files as COMPARE_EMBL_FILES } from './compare_embl_files.nf'

workflow {
    COMPARE_READERS(params.samplesheet) // compares the output of the two protgraph readers (rust and cpp) for a given set of inputs
    //COMPARE_EMBL_FILES(params.samplesheet) // compares the output of synthetic EMBL files EMBL files from UniProt
}

