@Library('jenkins-library') _

def pipeline = new org.rust.AppPipeline(steps: this,
      initSubmodules: true,
      envImageName: 'docker.soramitsu.co.jp/sora2/env:sub4',
      appImageName: 'docker.soramitsu.co.jp/sora2/bridge-relayer',
      // To prevent crash because there is no dockerfile
      pushTags: [:],
      buildTestCmds: ['cargo build'],
      // It is running in gh actions
      cargoClippyCmds: []
)
pipeline.runPipeline()
