
## PostgreSQL起動
$ docker-compose up postgres -d

## Pgadmin起動
$ docker-compose up pgadmin -d


## Pgadminログイン
以下のURLにアクセス
> http://localhost:18080/login    

mailおよびパスワードはdocker-compose.ymlで指定した以下の値
```
PGADMIN_DEFAULT_EMAIL
PGADMIN_DEFAULT_PASSWORD
```
### PgadminにログインしたらPostgresのコンテナに接続するように以下を実行
1. Server -> Register -> Server...
2. GeneralタブでNameには適当な値を入力
3. ConnectionタブのホストにはDockerネットワークで対象のコンテナにつながっているのでコンテナ名(postgres)を指定する。それからport(5432)およびUseName(postgres)の入力確認
4. SaveでServerGroupにPostgresコンテナが追加される。
5. Create Databaseにて"front", "dealing"のDatabaseを作成する。


## flyway実行
1. 事前にDatabaseを作成した状態で以下を実行    
2. postgresのコンテナ停止
<!-- 3. postgres/data/pg_hba.confに以下を追記    
    ```host all all all trust``` -->
4. Postgresコンテナ起動
5. 以下を実行しfront側のテーブル定義に反映    
   ``` docker-compose up flyway-front```
postgres/data/pg_hba.confに以下を追記
```
host all all all trust
```