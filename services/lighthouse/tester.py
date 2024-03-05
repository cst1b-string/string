import json
import pgpy
import base64
import requests

sig = 'aa'
key, _ = pgpy.PGPKey.from_file('../../tools/comm-test/key.asc')
endpoint_payload = {'endpoint': '193.60.93.228:22346',
                    'pubkey': str(key.pubkey),
                    'signature': sig,
                    'timestamp': 111
                    }
resp = requests.get('http://127.0.0.1:3000/wipe')
print(resp.content)
resp = requests.post('http://127.0.0.1:3000/register', json = endpoint_payload)
print(resp)
print(resp.content)
json_resp = json.loads(resp.content)
lookup_payload =   {'id': json_resp['id'],
                    'client': '1.1.1.1:9999',
                    'timestamp': 111}
resp = requests.post('http://127.0.0.1:3000/lookup', json = lookup_payload)
print(resp)
print(resp.content)
list_payload = {
    'id': json_resp['id'],
    'signature': sig,
    'timestamp': 111
}
resp = requests.post('http://127.0.0.1:3000/listconns', json = list_payload)
print(resp)
print(resp.content)
