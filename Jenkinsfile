@Library('jenkins-library') _

def pipeline = new org.rust.AppPipeline(steps: this,
      initSubmodules: true,
      envImageName: 'docker.soramitsu.co.jp/sora2/env:rust-1.73.0',
      appImageName: 'docker.soramitsu.co.jp/sora2/bridge-relayer',
      pushTags: [:],
      buildTestCmds: ['housekeeping/build.sh'],
      codeCoverageCommand: './housekeeping/coverage.sh',
      cargoClippyCmds: [],
      sonarProjectKey: 'sora:bridge-relayer',
      sonarProjectName: 'bridge-relayer',
      dojoProductType: 'sora'
)
pipeline.runPipeline()
