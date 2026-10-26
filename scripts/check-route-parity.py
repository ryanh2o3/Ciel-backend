#!/usr/bin/env python3
"""Fail if an HTTP method/path in any native API is absent from another."""
import pathlib,re
ROOT=pathlib.Path(__file__).resolve().parents[1]
def normalize(path):
    path=re.sub(r'\{(\w+):guid\}',r'{\1}',path)
    path=re.sub(r':(\w+)',r'{\1}',path)
    return path.rstrip('/') or '/'
def routes():
    rust=set()
    source=(ROOT/'src/http/routes.rs').read_text()
    for path,method in re.findall(r'\.route\(\s*"([^"]+)"\s*,\s*(get|post|patch|delete)\(',source):
        rust.add((method.upper(),normalize(path if path in ('/health','/metrics') else '/v1'+path)))
    spring=set()
    for file in (ROOT/'spring/src/main/java/com/ciel/web').glob('*Controller.java'):
        text=file.read_text()
        base=re.search(r'@RequestMapping\("([^"]+)"\)\s*public class',text)
        prefix=base[1] if base else ''
        for match in re.finditer(r'@(Get|Post|Patch|Delete)Mapping(?:\(([^\n]*)\))?',text):
            arg=match[2] or ''; path=re.search(r'"([^"]*)"',arg)
            spring.add((match[1].upper(),normalize(prefix+(path[1] if path else ''))))
    dotnet=set()
    for file in (ROOT/'dotnet/Ciel.Api/Endpoints').glob('*.cs'):
        text=file.read_text()
        if file.name=='ExtendedEndpoints.cs':
            for path,method in re.findall(r'group\s*\.MapMethods\("([^"]+)"\s*,\s*new\[\]\s*\{\s*"(\w+)"',text):
                dotnet.add((method,normalize('/v1'+path)))
        else:
            prefixes={'Auth':'/v1/auth','User':'/v1/users','Post':'/v1/posts','Feed':'/v1/feed','Media':'/v1/media','Health':''}
            group=file.stem.removesuffix('Endpoints')
            for method,path in re.findall(r'\.(?:Map)(Get|Post|Patch|Delete)\("([^"]*)"',text):
                dotnet.add((method.upper(),normalize(prefixes[group]+path)))
    for method,path in re.findall(r'app.Map(Get|Post|Patch|Delete)\("([^"]+)"',(ROOT/'dotnet/Ciel.Api/Program.cs').read_text()):
        dotnet.add((method.upper(),normalize(path)))
    return {'rust':rust,'spring':spring,'dotnet':dotnet}
if __name__=='__main__':
    implementations=routes();union=set.union(*implementations.values());failed=False
    for name,actual in implementations.items():
        missing=sorted(union-actual)
        print(f'{name}: {len(actual)} method/path pairs')
        for method,path in missing:print(f'  MISSING {method} {path}');failed=True
    raise SystemExit(1 if failed else 0)
