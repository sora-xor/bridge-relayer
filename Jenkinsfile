@Library('jenkins-library') _

def pipeline = new org.rust.AppPipeline(steps: this,
      initSubmodules: true,
      envImageName: 'docker.soramitsu.co.jp/sora2/env:env',
      appImageName: 'docker.soramitsu.co.jp/sora2/bridge-relayer',
      pushTags: ['master':'latest', 'evm-bridge': 'evm-bridge'],
      buildTestCmds: 'housekeeping/build.sh',
      codeCoverageCommand: './housekeeping/coverage.sh',
      clippyLinter: false,
      sonarProjectKey: 'sora:bridge-relayer',
      sonarProjectName: 'bridge-relayer',
      dojoProductType: 'sora'
)
pipeline.runPipeline()
