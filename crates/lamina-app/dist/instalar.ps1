# Instala o Lamina no Windows pelo terminal (spec DS, regra 14), sem administrador, na pasta do
# usuário:
#
#   irm https://raw.githubusercontent.com/alberthpalhares/lamina-albuns/main/crates/lamina-app/dist/instalar.ps1 | iex
#   $env:LAMINA_BETA = 1; irm .../instalar.ps1 | iex      (a versão de teste mais nova)
#
# O endereço "raw" diz que o arquivo é UTF-8; sem isso, o PowerShell 5.1 troca os acentos.
#
# O que o PowerShell baixa não ganha a marca de "baixado da internet", então o SmartScreen não
# mostra o aviso. O Lamina não fica verificado pela Microsoft: o aviso só não aparece, e o
# antivírus continua examinando. A soma SHA-256 do instalador é conferida antes de rodar.
#
# Funciona no Windows PowerShell 5.1, o que já vem no Windows. Para os testes: LAMINA_FONTE troca
# de onde vêm os arquivos (uma pasta ou um endereço) e LAMINA_TESTE=1 para depois de conferir.

param([switch]$Beta)

function Install-Lamina {
    param([bool]$Beta)
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    $repo = 'alberthpalhares/lamina-albuns'
    $semRede = 'Não foi possível baixar o Lamina. Confira a internet e tente de novo.'

    if (-not [Environment]::Is64BitOperatingSystem) {
        Write-Host 'Este sistema não é suportado pelo Lamina (precisa do Windows de 64 bits).' -ForegroundColor Red
        return $false
    }

    if ($env:LAMINA_FONTE) {
        $fonte = $env:LAMINA_FONTE
    } elseif ($Beta) {
        try {
            $releases = Invoke-RestMethod -UseBasicParsing "https://api.github.com/repos/$repo/releases?per_page=20"
            $fonte = "https://github.com/$repo/releases/download/$($releases[0].tag_name)"
        } catch {
            Write-Host $semRede -ForegroundColor Red
            return $false
        }
    } else {
        $fonte = "https://github.com/$repo/releases/latest/download"
    }

    $temp = Join-Path ([IO.Path]::GetTempPath()) ('lamina-instalar-' + [Guid]::NewGuid())
    New-Item -ItemType Directory -Path $temp | Out-Null
    try {
        $baixa = {
            param($nome)
            $alvo = Join-Path $temp $nome
            if (Test-Path -LiteralPath $fonte -PathType Container) {
                Copy-Item -LiteralPath (Join-Path $fonte $nome) -Destination $alvo
            } else {
                Invoke-WebRequest -UseBasicParsing -Uri "$fonte/$nome" -OutFile $alvo
            }
            $alvo
        }

        Write-Host 'Baixando o Lamina…'
        try {
            $somas = & $baixa 'SHA256SUMS'
        } catch {
            Write-Host $semRede -ForegroundColor Red
            return $false
        }
        $linha = Get-Content -LiteralPath $somas | Where-Object { $_ -match 'win.*Setup\.exe$' } | Select-Object -First 1
        if (-not $linha) {
            Write-Host 'Não foi possível baixar o Lamina: a versão publicada não tem o instalador do Windows.' -ForegroundColor Red
            return $false
        }
        $partes = $linha -split '\s+'
        $soma = $partes[0]
        $nome = $partes[1].TrimStart('*')
        try {
            $setup = & $baixa $nome
        } catch {
            Write-Host $semRede -ForegroundColor Red
            return $false
        }

        $conferida = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash
        if ($conferida -ne $soma.ToUpperInvariant()) {
            Remove-Item -LiteralPath $setup -Force
            Write-Host 'O arquivo baixado não confere; nada foi instalado.' -ForegroundColor Red
            return $false
        }
        if ($env:LAMINA_TESTE -eq '1') {
            Write-Host "conferido: $nome"
            return $true
        }

        Write-Host 'Instalando…'
        # Esperar só o instalador: ele abre o Lamina ao terminar, e a janela fica com a pessoa.
        $processo = Start-Process -FilePath $setup -ArgumentList '--silent' -PassThru
        $processo.WaitForExit()
        if ($processo.ExitCode -ne 0) {
            Write-Host "A instalação não terminou (código $($processo.ExitCode))." -ForegroundColor Red
            return $false
        }
        Write-Host 'Pronto! O Lamina foi instalado. Abra pelo atalho na área de trabalho ou no menu Iniciar.' -ForegroundColor Green
        return $true
    } finally {
        Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

$ok = Install-Lamina -Beta ($Beta -or $env:LAMINA_BETA -eq '1')
# Rodado de um arquivo (testes, CI), sai com o código; pelo `irm | iex`, não fecha a janela.
if ($PSCommandPath -and -not $ok) { exit 1 }
