import sqlite3


def create_db():
    try:
        conn = sqlite3.connect("credentials.db")
        c = conn.cursor()
        c.execute(''' CREATE TABLE credentials 
        (
        username TEXT PRIMARY KEY,
        password TEXT  
        )
        ''')
        conn.commit()
        return True
    except BaseException:
        return False
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()


def add_user(username, passwd):
    pass


def get_username(username):
    pass


def get_passwd(username):
    pass


def is_user_in_system(username):
    pass


def authenticate(hashed_pw, plaintext):
    pass

def hash_passwd(plaintext):
    pass


def sanitize(in_word):
    pass



