@Library('jenkins-library') _

def pipeline = new org.rust.AppPipeline(steps: this,
      initSubmodules: true,
      envImageName: 'docker.soramitsu.co.jp/sora2/env:sub4',
      appImageName: 'docker.soramitsu.co.jp/sora2/bridge-relayer',
      pushTags: [:],
      buildTestCmds: ['cargo build'],
      cargoClippyCmds: []
)
pipeline.runPipeline()
