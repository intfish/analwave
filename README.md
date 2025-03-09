# analwave

Crude tool to detect underruns and silence in WAV files. 
The primary use-case is for identifying problems and waste in large multi-track recording sessions.

```
Usage: analwave [OPTIONS] --input <INPUT>

Options:
  -i, --input <INPUT>
          The file to analyze
  -u, --underrun
          Detect underruns
      --samples <SAMPLES>
          Underrun detection minimum samples [default: 16]
  -s, --silence <SILENCE>
          Detect silence of at least given seconds [default: 0]
      --lufs <LUFS>
          Silence threshold (LUFS-S) [default: -70]
      --silence-percentage <SILENCE_PERCENTAGE>
          Silence percentage (returns error code if total silence is above this threshold) [default: 99]
      --debug
          Debug output
  -h, --help
          Print help
  -V, --version
          Print version
```


## Return codes

- If underruns are detected then `exit_code & 0b0001` will be true.
- If total silence amount exceeds --silence-percentage then `exit_code & 0b0010` will be true.
