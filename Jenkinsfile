@Library('jenkins-library') _

def pipeline = new org.rust.AppPipeline(steps: this,
      initSubmodules: true,
      envImageName: 'docker.soramitsu.co.jp/sora2/env:test',
      appImageName: 'docker.soramitsu.co.jp/sora2/bridge-relayer',
      pushTags: ['master':'latest', 'ton-bridge': 'ton-bridge', 'update-relayer': 'parachain-bridge'],
      buildTestCmds: 'housekeeping/build.sh',
      codeCoverageCommand: './housekeeping/coverage.sh',
      clippyLinter: false,
      sonarProjectKey: 'sora:bridge-relayer',
      sonarProjectName: 'bridge-relayer',
      dojoProductType: 'sora'
)
pipeline.runPipeline()
