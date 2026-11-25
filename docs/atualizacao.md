# Guia Passo a Passo — Atualização Atômica do Tamagi OS (`bootc` + GHCR)

> **Você não precisa reinstalar o sistema.**  
> O Tamagi é um sistema atômico (`bootc` + `composefs`): ao trocar ou atualizar a imagem OCI, sua pasta pessoal (`/var/home`), Flatpaks, devboxes, agentes de IA e configurações permanecem intactos.

---

## Parte 1 — Publicar as imagens assinadas no GitHub (GHCR)

O Tamagi exige assinatura criptográfica (`cosign`, chaves públicas em `cosign.pub` e `cosign-2.pub`) para aceitar qualquer imagem vinda de `ghcr.io/wes-tamagi`. O workflow `.github/workflows/build.yml` faz build, testes (`tests/image.sh`), publicação e assinatura automaticamente para as duas edições:

- **Tamagi-DxAI:** `ghcr.io/wes-tamagi/tamagi:latest`
- **Tamagi-SKL:** `ghcr.io/wes-tamagi/tamagi-skl:latest`

### 1.1 Subir a branch para o GitHub

No repositório local:

```bash
git push origin feat/v1.5-dxai-skl
```

*(Se pedir login no terminal, autentique antes com `gh auth login` ou configure sua chave SSH.)*

### 1.2 Conferir o secret de assinatura (`SIGNING_SECRET`)

No GitHub, acesse **Settings → Secrets and variables → Actions** no repositório `wes-tamagi/tamagi-OS` e confirme que existe:

- `SIGNING_SECRET`: conteúdo da chave privada correspondente a `cosign.pub` *(obrigatório — sem ela o workflow falha para não publicar uma imagem recusada pelas máquinas)*.
- `SIGNING_SECRET_2` *(opcional)*: chave privada reserva correspondente a `cosign-2.pub`.

### 1.3 Fazer o merge na branch `main`

O workflow `.github/workflows/build.yml` só publica no GHCR a partir da branch padrão (`main`):

1. Abra um Pull Request de `feat/v1.5-dxai-skl` → `main` (ou faça o merge na `main`).
2. Acompanhe a aba **Actions → build** até os dois jobs (`dxai` e `skl`) ficarem verdes.

*(Dica: também é possível disparar uma nova publicação manualmente em **Actions → build → Run workflow** na branch `main`.)*

### 1.4 Deixar os pacotes públicos no GitHub Packages

Na primeira publicação, o GHCR pode criar os pacotes como privados. Como o `bootc` roda como `root` sem credenciais de usuário, os pacotes precisam ser públicos:

1. Acesse o perfil/organização `wes-tamagi` no GitHub → aba **Packages**.
2. Abra o pacote **`tamagi`** → **Package settings** → **Change visibility** → **Public**.
3. Repita o mesmo para o pacote **`tamagi-skl`**.

---

## Parte 2 — Migrar uma máquina já instalada (uma única vez)

Se o computador foi instalado a partir de uma ISO local antiga que gravou `localhost/tamagi:latest`, rode **uma única vez** o comando correspondente à edição instalada:

### Tamagi-DxAI

```bash
sudo bootc switch ghcr.io/wes-tamagi/tamagi:latest
systemctl reboot
```

### Tamagi-SKL

```bash
sudo bootc switch ghcr.io/wes-tamagi/tamagi-skl:latest
systemctl reboot
```

### Conferir depois de reiniciar

```bash
sudo bootc status
tamagi sys doctor
```

No `bootc status`, o campo `Booted image` deve mostrar `ghcr.io/wes-tamagi/tamagi:latest` (ou `tamagi-skl:latest`).  
No `Tamagi-DxAI`, as novidades desta revisão (`mise` nativo, `tamagi remove`, `tamagi install --native chrome`, `tamagi-fboot`, `tamagi-agent` unificado) já estarão ativas imediatamente.

---

## Parte 3 — Atualizações do dia a dia

Depois do `bootc switch` inicial, não é mais preciso rodar `bootc` manualmente. O sistema atualiza em segundo plano (timer diário) ou sob demanda com um único comando:

```bash
tamagi update
```

Isso atualiza em sequência:
1. **Checkpoint automático** da pasta pessoal (`snapper`, para `tamagi rewind`);
2. **Imagem atômica do sistema** (`bootc` + verificação de Secure Boot e quarentena);
3. **Camadas nativas** (`tamagi-layer rebuild`, caso existam pacotes `--native` instalados);
4. **Flatpaks** (`--user`);
5. **Caixas Distrobox** (Fedora/Ubuntu/openSUSE/Arch);
6. **Agentes de IA**, **runtimes `mise`**, **Homebrew** (se instalado) e **Podman Quadlets (MCP)**.

### Comandos úteis do `tamagi update`

| Comando | O que faz |
|---|---|
| `tamagi update` | Atualiza tudo e prepara a nova imagem para o próximo boot |
| `tamagi update --reboot` | Atualiza tudo e reinicia automaticamente se houver nova imagem |
| `tamagi update --soft-reboot` | Aplica com soft-reboot (ou reboot completo se o kernel mudou) |
| `tamagi update --system` | Atualiza somente a imagem atômica do sistema |
| `tamagi update check` | Verifica se há nova imagem no GHCR sem baixar |
| `tamagi update changes` | Mostra o que muda na imagem pronta (kernel, COSMIC, pacotes) |
| `tamagi update status` | Exibe timers automáticos, imagem preparada e quarentena |
| `tamagi update drop` | Descarta uma atualização preparada antes de reiniciar |
| `tamagi update rollback` | Volta para a imagem anterior no próximo boot |
| `tamagi update auto on\|off` | Liga ou desliga as atualizações automáticas |
| `tamagi update quarantine` | Lista digests bloqueados após falha de saúde no boot |
| `tamagi update unquarantine` | Limpa a lista de quarentena |

---

## Parte 4 — Novas ISOs já apontam para o GHCR

A partir do commit `dd52268`, o `Justfile` já define como padrão:

- `TAMAGI_TARGET_IMGREF=ghcr.io/wes-tamagi/tamagi:latest`
- `TAMAGI_SKL_TARGET_IMGREF=ghcr.io/wes-tamagi/tamagi-skl:latest`

Portanto, qualquer ISO gerada com `just build-iso` continua instalando **100% offline** a partir do payload embutido na mídia, mas o sistema instalado já nasce apontando para o GHCR — dispensando o passo do `bootc switch`.

---

## Parte 5 — Solução rápida de problemas

| Sintoma | Causa provável | Como resolver |
|---|---|---|
| `dial tcp [::1]:443: connect: connection refused` (`localhost/tamagi:latest`) | Máquina instalada de ISO local sem `TARGET_IMGREF` | Execute a **Parte 2** (`sudo bootc switch ghcr.io/wes-tamagi/tamagi:latest`) |
| `unauthorized` ou `denied` ao baixar de `ghcr.io/wes-tamagi/...` | Pacote ainda está como privado no GitHub Packages | Execute o passo **1.4** e mude a visibilidade para **Public** |
| `Signature verification failed` / imagem recusada pela `policy.json` | Imagem publicada sem assinatura `cosign` válida | Garanta `SIGNING_SECRET` configurado no GitHub Actions e republique pelo workflow `build.yml` |
| Sistema voltou sozinho para a imagem anterior após reiniciar | A verificação de saúde (`tamagi-boot-check`) detectou falha e colocou o digest em quarentena | Veja o motivo em `tamagi sys boot-report show` e confira `tamagi update quarantine` |
