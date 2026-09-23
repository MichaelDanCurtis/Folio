"""Read-only guard for native preview hygiene; quit apps through their normal UI."""
import argparse,json,subprocess
p=argparse.ArgumentParser();p.add_argument('--expected',type=int,default=1);args=p.parse_args()
rows=subprocess.check_output(['ps','-axo','pid=,command='],text=True).splitlines()
apps=[]
for row in rows:
 parts=row.strip().split(None,1)
 if len(parts)!=2:continue
 pid,command=parts
 if '/Contents/MacOS/' in command and command.rsplit('/Contents/MacOS/',1)[-1] in {'Folio Native','Folio'}:
  apps.append({'pid':int(pid),'executable':command})
print(json.dumps({'instances':len(apps),'apps':apps},indent=2))
if len(apps)!=args.expected:raise SystemExit(f'Expected {args.expected} Folio instance(s), found {len(apps)}')
