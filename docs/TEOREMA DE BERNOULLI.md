-## Introducción

El Teorema de Bernoulli es un principio fundamental en la mecánica de fluidos que describe la relación entre la presión, la velocidad y la energía de un fluido en movimiento. Se basa en el principio de conservación de la energía, que establece que la energía total de un sistema aislado permanece constante a lo largo del tiempo. En el contexto de la mecánica de fluidos, este principio se expresa a través del Teorema de Bernoulli, donde la energía se transforma entre energía potential, energía cinética y energía de flujo.

Este teorema es applicable a fluidos incompresibles y no viscosos (es decir, sin rozamiento). Fue enunciado por el matemático y científico suizo Daniel Bernoulli en su obra _Hydrodynamica_ (1738).

El teorema establece que la energía mecánica total de un flujo incompresible y no viscoso es constante a lo largo de una línea de corriente. Una línea de corriente es una línea imaginaria que siempre es paralela a la dirección del flujo en cada punto. En un flujo uniforme, las líneas de corriente coinciden con la trayectoria de las partículas individuales del fluido.

El Teorema de Bernoulli implica una relación inversa entre la presión y la velocidad del fluido: cuando la velocidad aumenta, la presión disminuye, y vice-versa. Este principio tiene numerosas aplicaciones en diversas áreas de la ingeniería y la ciencia.

## Daniel Bernoulli: Un Breve Contexto Histórico

Daniel Bernoulli (1700-1782) fue un físico, matemático y médico suizo, miembro de una familia de destacados matemáticos. Nació en Groningen, Países Bajos, y falleció en Basilea, Suiza. Realizó importantes contribuciones en diversos campos de la ciencia, incluyendo la mecánica de fluidos, la probabilidad y la estadística. Su obra _Hydrodynamica_, publicada en 1738, sentó las bases de la dinámica de fluidos y presentó el teorema que lleva su nombre.

## Ecuación de Bernoulli

Considerando el caudal en dos secciones diferentes de una tubería y aplicando la ley de la conservación de la energía, la ecuación de Bernoulli se puede escribir como:

```
P₁ + (1/2)ρv₁² + ρgh₁ = P₂ + (1/2)ρv₂² + ρgh₂
```

Donde:

- `P` es la presión del fluido.
- `ρ` es la densidad del fluido (constante para fluidos incompresibles).
- `v` es la velocidad del fluido.
- `g` es la aceleración debida a la gravedad.
- `h` es la altura sobre un punto de referencia.

Cada término en la ecuación representa una forma de energía por unidad de volumen del fluido:

- `P`: Energía de presión, asociada a la presión del fluido.
- `(1/2)ρv²`: Energía cinética, asociada al movimiento del fluido.
- `ρgh`: Energía potential gravitacional, asociada a la altura del fluido.

La ecuación de Bernoulli establece que la suma de estos tres términos es constante a lo largo de una línea de corriente en un fluido ideal.

## Definiciones Clave

- **Energía Potential:** En el contexto de fluidos, es la energía asociada a la posición del fluido en un campo gravitacional. Se representa por el término `ρgh` en la ecuación de Bernoulli.
- **Energía Cinética:** Es la energía asociada al movimiento del fluido. Se representa por el término `(1/2)ρv²` en la ecuación de Bernoulli.
- **Energía de Flujo (o Energía de Presión):** Es la energía asociada a la presión del fluido, que representa el trabajo necesario para mover el fluido contra la presión circundante. Se representa por el término `P` en la ecuación de Bernoulli.
- **Altura Piezométrica:** Es la altura que alcanzaría una columna de fluido en un piezómetro (un tubo vertical conectado a la tubería). Representa la suma de la altura de presión (`P/ρg`) y la altura geométrica (`h`).
- **Tubo de Pitot:** Es un instrumento utilizado para medir la velocidad de un fluido. Mide la presión total (o de estancamiento) del fluido, que es la suma de la presión estática y la presión dinámica. Combinando la presión total medida con el tubo de Pitot y la presión estática, se puede calcular la velocidad del fluido utilizando la ecuación de Bernoulli.

## Aplicaciones del Teorema de Bernoulli

El Teorema de Bernoulli tiene numerosas aplicaciones en ingeniería y ciencia, incluyendo:

- **Diseño de Alas de Aviones (Aerodinámica):** La forma de un ala está diseñada para que el aire fluya más rápido sobre la superficie superior que sobre la inferior. Esto crea una diferencia de presión, generando una fuerza de sustentación que permite que el avión vuele.
- **Medidores Venturi:** Estos dispositivos se utilizan para medir la velocidad de flujo de un fluido en una tubería. Se basan en la constricción de la tubería, lo que aumenta la velocidad del fluido y disminuye la presión. La diferencia de presión se utilize para calcular la velocidad de flujo.
- **Carburadores:** En motores de combustión interna, los carburadores utilizan el Teorema de Bernoulli para mezclar aire y combustible. La disminución de presión en el venturi del carburador aspira el combustible hacia la corriente de aire.
- **Atomizadores:** Dispositivos como los pulverizadores de pintura o los atomizadores de perfume utilizan el Teorema de Bernoulli para crear una fina pulverización. La alta velocidad del aire en la boquilla reduce la presión, permitiendo que el líquido se descomponga en pequeñas gotas.
- **Flujo Sanguíneo:** En medicina, el Teorema de Bernoulli se utilize para estimar la velocidad del flujo sanguíneo utilizando un flujómetro Doppler.

## Equipo y Materiales Requeridos (del documento original)

1.  Módulo básico Teorema de Bernoulli Edibon FME 03.
2.  Banco hidráulico Eibon FME 00
3.  Cronómetro

## Desarrollo de la Práctica (del documento original)

### Llenado de los tubos manométricos.

1.  Suministrar caudal al sistema mediante el banco hidráulico, al máximo hasta que los manómetros estén llenos y sin vacíos.
2.  Cerrar la válvula de control del equipo Teorema de Bernoulli (VCC)
3.  Cerrar la válvula de control del banco hidráulico (VC)
4.  Abrir la válvula de purga.
5.  Abrir despacio la válvula de control VCC, observar como los tubos comienzan a llenarse de aire.
6.  Cuando todos los tubos han obtenido la altura deseada (70 – 80 mm), cerrar la válvula VCC y cerrar la válvula de purga.
7.  En este memento todos los tubos tienen el mismo nivel de agua.

### Determinación exacta del tubo Venturi:

1.  Abrir la válvula VCC y VC al mismo tiempo, lentamente hasta fijar un caudal y anotar su valor.
2.  Colocar el tubo de Pitot en la primera toma de presión de mínima sección. Esperar a que la altura en el tubo manométrico de Pitot se estabilice. Este proceso puede tardar unos minutos.
3.  Cuando la altura de ambos tubos sea estable, determinar la diferencia de altura entre los dos tubos manométricos; presión estática ℎ2 y presión total ℎ3 del tubo de Pitot.
4.  La diferencia corresponde a la presión cinética dada por 
     .
5.  Determinar la sección con la siguiente ecuación: 4
    5
    donde Q es el caudal del fluido y V es la velocidad obtenida en dicha sección, la cual se obtiene con la ecuación N°3.
6.  Repetir todos los pasos descritos anteriormente para cada toma de presión.
7.  Repetir los pasos previous para diferentes caudales de agua.
8.  Para cada caudal de agua la sección debe set más o menos la misma.

Calcular la medida de las secciones obtenidas con diferentes caudales de agua.
Se recomiendan caudales de agua de 5 l/min, 10 l/min y 15 l/min para la práctica.
Con esos valores llenar las siguientes tablas del anexo 1.

## Anexo N° 1 – Práctica: Teorema de Bernoulli (del documento original)

### REGISTRO DE CAUDALES

| Caudal (litros) | 8 Tiempo (min) | 8 Tiempo (min) | 89 Tiempo (min) |
| --------------- | -------------- | -------------- | --------------- |
| 0/ 5            |                |                |                 |
| 5/ 10           |                |                |                 |
| 10 / 15         |                |                |                 |
| 15 / 20         |                |                |                 |
| 20 / 25         |                |                |                 |
| 25 / 30         |                |                |                 |
| 30 / 35         |                |                |                 |
| 35 / 40         |                |                |                 |

_Nota: Promediar los tiempos y el resultado en minutos será el divisor de 5, hasta obtener los caudales sugeridos, 5 [[/]]) , 10 [[/]]) y 15 [[/]])_

### REGISTRO DE ALTURAS PIEZOMÉTRICAS

|     | 8   | 8   | 89  |
| --- | --- | --- | --- | --- | --- | --- |
|     | ℎ2  | ℎ3  | ℎ2  | ℎ3  | ℎ2  | ℎ3  |
| 1   |     |     |     |     |     |     |
| 2   |     |     |     |     |     |     |
| 3   |     |     |     |     |     |     |
| 4   |     |     |     |     |     |     |
| 5   |     |     |     |     |     |     |
| 6   |     |     |     |     |     |     |

_Realizar una gráfica de las presiones estáticas, totales y dinámicas. ℎ@2A ℎ3 /ℎ 2_

### CÁLCULO DE LA VELOCIDAD

|         | 8 5 [[/]])  02 ∙!∙+ℎ3 / ℎ2- | 8 10 [[/]])  02 ∙!∙+ℎ3 / ℎ2- | 8 15 [[/]]) 9 02 ∙!∙+ℎ3 / ℎ2- |
| ------- | --------------------------- | ---------------------------- | ----------------------------- |
| ℎ3 / ℎ  |                             |                              |                               |
| ℎ3 / ℎ  |                             |                              |                               |
| ℎ3 / ℎ9 |                             |                              |                               |
| ℎ3 / ℎB |                             |                              |                               |
| ℎ3 / ℎC |                             |                              |                               |
| ℎ3 / ℎD |                             |                              |                               |

### CÁLCULO DEL ÁREA (Sección transversal tubo Venturi)

| | " 8
1 | " 8
2 | "9 89
3 | """"9
3 |
|---|---|---|---|---|
| ℎ3 / ℎ | | | | |
| ℎ3 / ℎ | | | | |
| ℎ3 / ℎ9 | | | | |
| ℎ3 / ℎB | | | | |
| ℎ3 / ℎC | | | | |
| ℎ3 / ℎD | | | | |

_✔ Las áreas ",","9 son similares o diferentes, ¿A qué se debe esto?_

### DEMOSTRACIÓN DEL TEOREMA DE BERNOULLI

ℎ
 

2! ℎ  

2! ℎ 9 9

2! ℎ B B

2! ℎ C C

2! ℎ D D

2!

_✔ ¿Se cumple la igualdad?_

```