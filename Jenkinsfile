@Library('jenkins-library') _

def pipeline = new org.rust.AppPipeline(steps: this,
      initSubmodules: true,
      envImageName: 'docker.soramitsu.co.jp/sora2/env:rust-1.73.0',
      appImageName: 'docker.soramitsu.co.jp/sora2/bridge-relayer',
      pushTags: ['liberland':'sora-liberland'],
      buildTestCmds: ['housekeeping/build.sh'],
      cargoClippyCmds: []
)
pipeline.runPipeline()
