#!/usr/bin/env python3
"""
asc_cert.py — App Store Connect API で配布証明書を作り、.p12 に束ねる。

なぜ API 経由か: Xcode が作る配布証明書は**データ保護キーチェーン**に入るため、
`security export` では取り出せない（秘密鍵が出せないので .p12 を作れない）。
ここでは秘密鍵をこちらで生成し、CSR を API に投げて証明書を受け取るので、
鍵と証明書の両方が手元に揃ひ、CI へ持ち込める形になる。

    python3 scripts/asc_cert.py <KEY_ID> <ISSUER_ID> <AuthKey_XXX.p8> <out.p12> <p12password>

依存: cryptography（標準の pip 環境に入つてゐる）
"""
from __future__ import annotations

import base64
import json
import os
import subprocess
import tempfile
import sys
import time
import urllib.request

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec, rsa, utils as asym_utils
from cryptography.x509.oid import NameOID

API = "https://api.appstoreconnect.apple.com/v1"


def make_jwt(key_id: str, issuer_id: str, p8_path: str) -> str:
    """ASC API 用の ES256 JWT。署名は DER でなく **raw r||s**（64 バイト）でなければならない。"""
    with open(p8_path, "rb") as f:
        key = serialization.load_pem_private_key(f.read(), password=None)
    if not isinstance(key, ec.EllipticCurvePrivateKey):
        raise SystemExit("✗ .p8 が EC 鍵ではありません（ASC API キーではない可能性）")

    def b64(d: bytes) -> bytes:
        return base64.urlsafe_b64encode(d).rstrip(b"=")

    now = int(time.time())
    header = {"alg": "ES256", "kid": key_id, "typ": "JWT"}
    payload = {"iss": issuer_id, "iat": now, "exp": now + 15 * 60,
               "aud": "appstoreconnect-v1"}
    signing_input = b".".join([
        b64(json.dumps(header, separators=(",", ":")).encode()),
        b64(json.dumps(payload, separators=(",", ":")).encode()),
    ])
    der = key.sign(signing_input, ec.ECDSA(hashes.SHA256()))
    r, s = asym_utils.decode_dss_signature(der)
    raw = r.to_bytes(32, "big") + s.to_bytes(32, "big")
    return (signing_input + b"." + b64(raw)).decode()


def call(token: str, path: str, method: str = "GET", body: dict | None = None) -> dict:
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(API + path, data=data, method=method)
    req.add_header("Authorization", "Bearer " + token)
    if data:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            return json.loads(r.read() or b"{}")
    except urllib.error.HTTPError as e:
        detail = e.read().decode(errors="replace")
        raise SystemExit(f"✗ API {method} {path} → HTTP {e.code}\n{detail[:600]}")


def main() -> None:
    if len(sys.argv) != 6:
        raise SystemExit(__doc__)
    key_id, issuer_id, p8, out_p12, p12_pass = sys.argv[1:6]
    token = make_jwt(key_id, issuer_id, p8)

    # 既存の配布証明書を確認（Apple は種類ごとに枚数制限があるため）
    got = call(token, "/certificates?filter[certificateType]=DISTRIBUTION&limit=200")
    existing = got.get("data", [])
    print(f"▶ 既存の配布証明書: {len(existing)} 枚")
    for c in existing:
        a = c["attributes"]
        print(f"    - {a.get('displayName')} / 期限 {a.get('expirationDate')}")

    # 秘密鍵は**こちらで**作る（さうしないと .p12 に束ねられない）。
    # Apple の証明書は RSA 2048 を要求する。
    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    csr = (x509.CertificateSigningRequestBuilder()
           .subject_name(x509.Name([
               x509.NameAttribute(NameOID.COMMON_NAME, "Yatate CI Distribution"),
               x509.NameAttribute(NameOID.COUNTRY_NAME, "US"),
           ]))
           .sign(key, hashes.SHA256()))
    csr_pem = csr.public_bytes(serialization.Encoding.PEM).decode()

    print("▶ 配布証明書を発行します")
    res = call(token, "/certificates", "POST", {
        "data": {"type": "certificates",
                 "attributes": {"certificateType": "DISTRIBUTION",
                                "csrContent": csr_pem}}}
    )
    attrs = res["data"]["attributes"]
    cert_der = base64.b64decode(attrs["certificateContent"])
    cert = x509.load_der_x509_certificate(cert_der)
    print(f"✓ 発行: {attrs.get('displayName')} / 期限 {attrs.get('expirationDate')}")
    print(f"  subject: {cert.subject.rfc4514_string()}")

    # macOS の `security import` は **旧方式の PKCS#12**（3DES + SHA1 MAC）しか
    # 受け付けない。cryptography が既定で使ふ PBES2/AES で束ねると
    # 「MAC verification failed」で弾かれる（実際に CI で踏んだ）。
    # よつて PEM を経由し、openssl に旧方式で束ねさせる。
    with tempfile.TemporaryDirectory() as td:
        key_pem = os.path.join(td, "key.pem")
        cert_pem = os.path.join(td, "cert.pem")
        with open(key_pem, "wb") as f:
            f.write(key.private_bytes(
                serialization.Encoding.PEM,
                serialization.PrivateFormat.PKCS8,
                serialization.NoEncryption()))
        with open(cert_pem, "wb") as f:
            f.write(cert.public_bytes(serialization.Encoding.PEM))
        cmd = ["openssl", "pkcs12", "-export",
               "-inkey", key_pem, "-in", cert_pem,
               "-out", out_p12, "-name", "Yatate CI Distribution",
               "-passout", "pass:" + p12_pass,
               "-keypbe", "PBE-SHA1-3DES", "-certpbe", "PBE-SHA1-3DES",
               "-macalg", "sha1"]
        r = subprocess.run(cmd, capture_output=True, text=True)
        if r.returncode != 0:
            # OpenSSL 3 系では旧方式に -legacy プロバイダが要る
            r = subprocess.run(cmd + ["-legacy"], capture_output=True, text=True)
            if r.returncode != 0:
                raise SystemExit("✗ .p12 の作成に失敗:\n" + r.stderr[:400])
    print(f"✓ .p12 を書き出しました（旧方式・macOS が取り込める形）: {out_p12}")


if __name__ == "__main__":
    main()
