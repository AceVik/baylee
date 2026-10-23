"""GPU consumer for immutable packages prepared concurrently on the CPU.

Keeps one training job at a time. Adam state transfers across packages while
recurrent lanes reset; a fixed independent validation set is reused. Every
package is one full epoch. Failed jobs and all checkpoints remain available.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import sys
import time

from run_guard4 import atomic_json, run_job


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--packages",type=Path,required=True)
    ap.add_argument("--producer-job",type=Path,required=True)
    ap.add_argument("--bootstrap-job",type=Path,required=True)
    ap.add_argument("--bootstrap-model",type=Path,required=True)
    ap.add_argument("--initial-model",type=Path,help="Optional original weights if the bootstrap regresses on fixed validation")
    ap.add_argument("--initial-validation",type=Path,help="Completed fixed-protocol initial validation JSON")
    ap.add_argument("--validation-data",type=Path,required=True)
    ap.add_argument("--out",type=Path,required=True)
    ap.add_argument("--active-status",type=Path)
    ap.add_argument("--timeout",type=float,default=43200)
    args=ap.parse_args()
    args.out.mkdir(exist_ok=True)
    runtime=Path(__file__).parent
    deadline=time.monotonic()+args.timeout

    def status(phase,**details):
        value={"root":str(args.out),"phase":phase,"promoted":False,"updated_unix":time.time(),**details}
        atomic_json(args.out/"status.json",value)
        if args.active_status:atomic_json(args.active_status,value)
        print(json.dumps(value),flush=True)

    def wait_for_bootstrap():
        while time.monotonic()<deadline:
            job=json.loads(args.bootstrap_job.read_text())
            if job["status"]=="complete":return
            if job["status"] not in ("starting","running"):raise RuntimeError("Bootstrap training failed")
            time.sleep(2)
        raise TimeoutError("Bootstrap deadline")

    try:
        status("waiting-for-current-training")
        wait_for_bootstrap()
        previous=args.bootstrap_model/"last.pt"
        if args.initial_model:
            if not args.initial_validation:raise ValueError("Initial-model selection requires matched validation")
            reference=json.loads(args.initial_validation.read_text())["metrics"]
            candidate=json.loads((args.bootstrap_model/"meta.json").read_text())["val_metrics"]
            score=lambda row:row["val_pol_loss"]+2*row["val_mse"]
            choice={"initial_score":score(reference),"bootstrap_score":score(candidate),
                    "criterion":"policy loss + 2 * value MSE on identical validation; not a strength claim"}
            if score(reference)<=score(candidate):
                previous=None
                choice["selected"]=str(args.initial_model)
            else:choice["selected"]=str(args.bootstrap_model)
            atomic_json(args.out/"bootstrap-selection.json",choice)
        used=[]
        if (args.out/"progress.json").exists():
            progress=json.loads((args.out/"progress.json").read_text())
            used=progress["consumed"];previous=Path(progress["last_checkpoint"])
        while time.monotonic()<deadline:
            ready=sorted(p.parent for p in args.packages.glob("package-*/ready.json"))
            pending=[p for p in ready if p.name not in used]
            if not pending:
                if (args.packages/"complete.json").exists():
                    status("all-packages-trained",last_checkpoint=str(previous),packages=len(used))
                    return
                producer=json.loads(args.producer_job.read_text())
                if producer["status"] not in ("starting","running","complete"):
                    raise RuntimeError("CPU package producer failed; inspect its logs")
                status("waiting-for-next-package",completed_packages=len(used))
                time.sleep(10)
                continue
            package=pending[0]
            marker=json.loads((package/"ready.json").read_text())
            if marker["metadata_sha256"]!=hashlib.sha256((package/"meta.json").read_bytes()).hexdigest():
                raise ValueError("Published package metadata changed")
            model=args.out/package.name
            command=[sys.executable,"-u",str(runtime/"train4.py"),"--data",str(package),
                "--validation-data",str(args.validation_data),"--out",str(model),
                *(["--init-state",str(previous)] if previous else ["--init",str(args.initial_model)]),
                "--d-model","640","--heads","10","--layers","8","--batch-size","8",
                "--validation-batch-size","8","--unroll","32","--epochs","1","--lr","0.00002",
                "--seed",str(2719+len(used)),"--validation-every","0","--checkpoint-every","100",
                "--thinking-steps","1","--thinking-curriculum","--activation-checkpointing",
                "--board-chunk","8","--decision-chunk","4","--cache-thinking-kv",
                "--prune-unused-policy-heads","--compile-regions","--num-workers","2","--async-checkpoint"]
            status("training-package",package=str(package),model=str(model),frames=marker["frames"])
            job_dir=args.out/"jobs"/package.name
            if run_job(job_dir,command,21600,32,[package/"meta.json",package/"ready.json",previous or args.initial_model/"net.pt",
                      args.validation_data/"meta.json",runtime/"sources.json"]):
                raise RuntimeError(f"Training failed: {job_dir}")
            previous=model/"last.pt";used.append(package.name)
            atomic_json(args.out/"progress.json",{"consumed":used,"last_checkpoint":str(previous)})
            atomic_json(package/"consumed.json",{"job":str(job_dir/"job.json"),"checkpoint":str(previous)})
        raise TimeoutError("Package consumer deadline")
    except BaseException as error:
        status("package-training-failed",error=str(error))
        raise


if __name__=="__main__":main()
